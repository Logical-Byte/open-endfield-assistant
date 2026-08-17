//! 可续传的完整更新包下载器。
//!
//! 本模块只负责 HTTP 与 `artifact.zip.part`：它不理解 Tauri，也不决定何时安装。
//! 续传必须同时具备本地部分文件和服务端 validator（校验标识），并使用
//! `Range` + `If-Range`，避免把两个不同版本的 ZIP 拼在一起。

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    time::Instant,
};

use anyhow::{Context, Result, bail};
use reqwest::{StatusCode, blocking::Client, header};

use super::transaction::HttpValidator;

/// 一次下载所需的稳定输入。
///
/// `validator` 来自上一次响应，并随事务持久化；没有校验标识时即使部分文件存在，
/// 也会从零下载，因为客户端无法证明远端文件仍是同一个对象。
pub struct DownloadRequest {
    /// 完整更新包 URL。
    pub url: String,
    /// 未完成归档的固定落盘位置。
    pub part_path: PathBuf,
    /// 上一次响应的 `ETag` 或 `Last-Modified`。
    pub validator: Option<HttpValidator>,
}

/// 下载器向工作流报告的一次进度快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// 文件当前总长度；有效续传时包含此前已下载的字节。
    pub downloaded_bytes: u64,
    /// 服务端可推导总长度时为 `Some`，未知时必须保持 `None`。
    pub total_bytes: Option<u64>,
    /// 本次进程新传输字节的平均速度，不包含旧的部分文件。
    pub bytes_per_second: u64,
}

/// 下载完成后需要写回事务的 HTTP 元数据。
pub struct DownloadResult {
    /// 最终响应给出的校验标识，供下次中断续传使用。
    pub validator: Option<HttpValidator>,
    /// 最终响应声明或推导出的归档总长度。
    pub total_bytes: Option<u64>,
}

/// 下载完整包，并通过两个回调把持久化时机和 UI 进度交给上层。
///
/// `response_ready` 在写正文前调用，使工作流先保存校验标识；`report` 可高频调用，
/// 由工作流负责节流。生产代码使用单调时钟 `Instant::now` 计算速度。
pub fn download(
    client: &Client,
    request: &DownloadRequest,
    response_ready: impl FnMut(Option<HttpValidator>, Option<u64>, bool) -> Result<()>,
    report: impl FnMut(Progress),
) -> Result<DownloadResult> {
    download_with_clock(client, request, response_ready, report, Instant::now)
}

/// `download` 的可控时钟实现。
///
/// 单独传入 `now` 只为了让速度测试不依赖真实网络耗时；下载行为与公开入口相同。
fn download_with_clock(
    client: &Client,
    request: &DownloadRequest,
    mut response_ready: impl FnMut(Option<HttpValidator>, Option<u64>, bool) -> Result<()>,
    mut report: impl FnMut(Progress),
    mut now: impl FnMut() -> Instant,
) -> Result<DownloadResult> {
    if let Some(parent) = request.part_path.parent() {
        fs::create_dir_all(parent).context("创建下载目录失败")?;
    }
    // 只有“部分文件 + 对应校验标识”同时存在时才有资格尝试续传。
    let partial_len = fs::metadata(&request.part_path).map_or(0, |metadata| metadata.len());
    let can_resume = partial_len > 0 && request.validator.is_some();
    let mut builder = client.get(&request.url);
    if can_resume {
        builder = builder.header(header::RANGE, format!("bytes={partial_len}-"));
        if let Some(validator) = request.validator.as_ref() {
            builder = builder.header(header::IF_RANGE, validator.value());
        }
    }
    let mut response = builder.send().context("下载请求失败")?;
    if !response.status().is_success() {
        bail!("下载请求返回 HTTP {}", response.status());
    }

    // `206` 只表示服务端接受了 `Range`。校验标识也必须未变化，才能安全追加。
    let initial_validator = response_validator(&response);
    let resumed = can_resume
        && response.status() == StatusCode::PARTIAL_CONTENT
        && initial_validator == request.validator;
    if can_resume && response.status() == StatusCode::PARTIAL_CONTENT && !resumed {
        // 服务端返回了另一份对象的区间：丢弃该响应，再发一次不带 Range 的完整请求。
        response = client
            .get(&request.url)
            .send()
            .context("validator 变化后重新下载失败")?;
        if !response.status().is_success() || response.status() == StatusCode::PARTIAL_CONTENT {
            bail!("重新下载请求返回 HTTP {}", response.status());
        }
    }

    let start = if resumed { partial_len } else { 0 };
    let validator = response_validator(&response);
    let total = total_size(&response, start);
    response_ready(validator.clone(), total, resumed)?;
    report(Progress {
        downloaded_bytes: start,
        total_bytes: total,
        bytes_per_second: 0,
    });

    // 只有已验证的续传才 `append`；其他情况 `truncate`，保证旧字节不会混入新归档。
    let mut output = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&request.part_path)
        .context("打开部分归档失败")?;
    let started = now();
    let mut transferred = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = response.read(&mut buffer).context("读取下载响应失败")?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .context("写入部分归档失败")?;
        transferred += count as u64;
        // `transferred` 从零开始，仅累计本次运行读到的正文，因此续传速度不会虚高。
        let elapsed = now().duration_since(started).as_secs_f64();
        report(Progress {
            downloaded_bytes: start + transferred,
            total_bytes: total,
            bytes_per_second: if elapsed > 0.0 {
                (transferred as f64 / elapsed) as u64
            } else {
                0
            },
        });
    }
    output.flush().context("刷新部分归档失败")?;
    if let Some(expected) = total {
        let actual = start + transferred;
        if actual != expected {
            bail!("下载大小不匹配: 期望 {expected}，实际 {actual}");
        }
    }
    Ok(DownloadResult {
        validator,
        total_bytes: total,
    })
}

impl HttpValidator {
    /// 返回可直接放入 `If-Range` 请求头的原始 header 值。
    fn value(&self) -> &str {
        match self {
            Self::Etag(value) | Self::LastModified(value) => value,
        }
    }
}

/// 优先读取 `ETag`，否则读取 `Last-Modified`，并保留校验标识的具体种类。
fn response_validator(response: &reqwest::blocking::Response) -> Option<HttpValidator> {
    response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .map(|value| HttpValidator::Etag(value.to_string()))
        .or_else(|| {
            response
                .headers()
                .get(header::LAST_MODIFIED)
                .and_then(|value| value.to_str().ok())
                .map(|value| HttpValidator::LastModified(value.to_string()))
        })
}

/// 从 `Content-Range` 或 `Content-Length` 推导完整文件大小。
///
/// 普通响应的 `Content-Length` 是本次正文长度；续传响应必须使用
/// `Content-Range: bytes start-end/total` 中的 `total`。
fn total_size(response: &reqwest::blocking::Response, start: u64) -> Option<u64> {
    if response.status() == StatusCode::PARTIAL_CONTENT {
        response
            .headers()
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.rsplit_once('/'))
            .and_then(|(_, total)| total.parse().ok())
    } else {
        response.content_length().map(|length| start + length)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Cursor,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };

    use tiny_http::{Header, Response, Server, StatusCode};

    use super::{DownloadRequest, HttpValidator, Progress, download, download_with_clock};

    fn header(name: &'static [u8], value: &'static [u8]) -> Header {
        Header::from_bytes(name, value).unwrap()
    }

    #[test]
    fn valid_validator_resumes_partial_file_and_progress() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let observed = Arc::new(Mutex::new(Vec::new()));
        let observed_server = Arc::clone(&observed);
        let handle = thread::spawn(move || {
            let request = server.recv().unwrap();
            observed_server.lock().unwrap().extend(
                request
                    .headers()
                    .iter()
                    .map(|h| (h.field.to_string(), h.value.to_string())),
            );
            request
                .respond(
                    Response::from_data(b"world".to_vec())
                        .with_status_code(StatusCode(206))
                        .with_header(header(b"ETag", b"\"v1\""))
                        .with_header(header(b"Content-Range", b"bytes 6-10/11")),
                )
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let part = temp.path().join("artifact.zip.part");
        fs::write(&part, b"hello ").unwrap();
        let mut progress = Vec::<Progress>::new();

        let result = download(
            &reqwest::blocking::Client::new(),
            &DownloadRequest {
                url: address,
                part_path: part.clone(),
                validator: Some(HttpValidator::Etag("\"v1\"".into())),
            },
            |_, _, _| Ok(()),
            |value| progress.push(value),
        )
        .unwrap();
        handle.join().unwrap();

        assert_eq!(fs::read(part).unwrap(), b"hello world");
        assert_eq!(progress.first().unwrap().downloaded_bytes, 6);
        assert_eq!(result.total_bytes, Some(11));
        let headers = observed.lock().unwrap();
        assert!(
            headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("Range") && value == "bytes=6-")
        );
        assert!(
            headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("If-Range") && value == "\"v1\"")
        );
    }

    #[test]
    fn rejected_resume_restarts_from_zero_without_appending() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(
                    Response::from_data(b"fresh".to_vec()).with_header(header(b"ETag", b"\"v2\"")),
                )
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let part = temp.path().join("artifact.zip.part");
        fs::write(&part, b"stale").unwrap();
        let mut progress = Vec::<Progress>::new();

        download(
            &reqwest::blocking::Client::new(),
            &DownloadRequest {
                url: address,
                part_path: part.clone(),
                validator: Some(HttpValidator::Etag("\"v1\"".into())),
            },
            |_, _, _| Ok(()),
            |value| progress.push(value),
        )
        .unwrap();
        handle.join().unwrap();

        assert_eq!(fs::read(part).unwrap(), b"fresh");
        assert_eq!(progress.first().unwrap().downloaded_bytes, 0);
    }

    #[test]
    fn last_modified_validator_can_resume_a_partial_file() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let request = server.recv().unwrap();
            assert!(request.headers().iter().any(|header| {
                header.field.equiv("If-Range")
                    && header.value.as_str() == "Mon, 17 Aug 2026 12:00:00 GMT"
            }));
            request
                .respond(
                    Response::from_data(b"world".to_vec())
                        .with_status_code(StatusCode(206))
                        .with_header(header(b"Last-Modified", b"Mon, 17 Aug 2026 12:00:00 GMT"))
                        .with_header(header(b"Content-Range", b"bytes 6-10/11")),
                )
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let part = temp.path().join("artifact.zip.part");
        fs::write(&part, b"hello ").unwrap();

        download(
            &reqwest::blocking::Client::new(),
            &DownloadRequest {
                url: address,
                part_path: part.clone(),
                validator: Some(HttpValidator::LastModified(
                    "Mon, 17 Aug 2026 12:00:00 GMT".into(),
                )),
            },
            |_, _, resumed| {
                assert!(resumed);
                Ok(())
            },
            |_| {},
        )
        .unwrap();
        handle.join().unwrap();

        assert_eq!(fs::read(part).unwrap(), b"hello world");
    }

    #[test]
    fn changed_validator_on_partial_response_restarts_with_a_full_request() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let resumed = server.recv().unwrap();
            resumed
                .respond(
                    Response::from_data(b"wrong".to_vec())
                        .with_status_code(StatusCode(206))
                        .with_header(header(b"ETag", b"\"v2\""))
                        .with_header(header(b"Content-Range", b"bytes 5-9/10")),
                )
                .unwrap();
            let restarted = server.recv().unwrap();
            assert!(
                restarted
                    .headers()
                    .iter()
                    .all(|header| !header.field.equiv("Range"))
            );
            restarted
                .respond(
                    Response::from_data(b"fresh".to_vec()).with_header(header(b"ETag", b"\"v2\"")),
                )
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let part = temp.path().join("artifact.zip.part");
        fs::write(&part, b"stale").unwrap();
        let mut progress = Vec::<Progress>::new();

        download(
            &reqwest::blocking::Client::new(),
            &DownloadRequest {
                url: address,
                part_path: part.clone(),
                validator: Some(HttpValidator::Etag("\"v1\"".into())),
            },
            |_, _, resumed| {
                assert!(!resumed);
                Ok(())
            },
            |value| progress.push(value),
        )
        .unwrap();
        handle.join().unwrap();

        assert_eq!(fs::read(part).unwrap(), b"fresh");
        assert_eq!(progress.first().unwrap().downloaded_bytes, 0);
    }

    #[test]
    fn resumed_speed_counts_only_bytes_transferred_in_this_run() {
        let partial_len = 1024 * 1024;
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(
                    Response::from_data(b"fresh".to_vec())
                        .with_status_code(StatusCode(206))
                        .with_header(header(b"ETag", b"\"v1\""))
                        .with_header(header(b"Content-Range", b"bytes 1048576-1048580/1048581")),
                )
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let part = temp.path().join("artifact.zip.part");
        fs::write(&part, vec![b'x'; partial_len]).unwrap();
        let mut progress = Vec::<Progress>::new();

        let started = Instant::now();
        let mut clock_calls = 0;
        download_with_clock(
            &reqwest::blocking::Client::new(),
            &DownloadRequest {
                url: address,
                part_path: part,
                validator: Some(HttpValidator::Etag("\"v1\"".into())),
            },
            |_, _, _| Ok(()),
            |value| progress.push(value),
            || {
                clock_calls += 1;
                if clock_calls == 1 {
                    started
                } else {
                    started + Duration::from_millis(100)
                }
            },
        )
        .unwrap();
        handle.join().unwrap();

        let final_progress = progress.last().unwrap();
        assert_eq!(final_progress.downloaded_bytes, partial_len as u64 + 5);
        assert!(final_progress.bytes_per_second < 10_000);
    }

    #[test]
    fn unknown_total_stays_unknown_and_progress_is_monotonic() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(Response::new(
                    StatusCode(200),
                    Vec::new(),
                    Cursor::new(b"unknown length".to_vec()),
                    None,
                    None,
                ))
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let part = temp.path().join("artifact.zip.part");
        let mut progress = Vec::<Progress>::new();

        let result = download(
            &reqwest::blocking::Client::new(),
            &DownloadRequest {
                url: address,
                part_path: part,
                validator: None,
            },
            |_, total, _| {
                assert_eq!(total, None);
                Ok(())
            },
            |value| progress.push(value),
        )
        .unwrap();
        handle.join().unwrap();

        assert_eq!(result.total_bytes, None);
        assert!(progress.iter().all(|value| value.total_bytes.is_none()));
        assert!(
            progress
                .windows(2)
                .all(|pair| pair[0].downloaded_bytes <= pair[1].downloaded_bytes)
        );
    }
}
