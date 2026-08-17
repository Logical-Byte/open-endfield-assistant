use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    time::Instant,
};

use anyhow::{Context, Result, bail};
use reqwest::{StatusCode, blocking::Client, header};

use super::transaction::HttpValidator;

pub struct DownloadRequest {
    pub url: String,
    pub part_path: PathBuf,
    pub validator: Option<HttpValidator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: u64,
}

pub struct DownloadResult {
    pub validator: Option<HttpValidator>,
    pub total_bytes: Option<u64>,
}

pub fn download(
    client: &Client,
    request: &DownloadRequest,
    mut response_ready: impl FnMut(Option<HttpValidator>, Option<u64>, bool) -> Result<()>,
    mut report: impl FnMut(Progress),
) -> Result<DownloadResult> {
    if let Some(parent) = request.part_path.parent() {
        fs::create_dir_all(parent).context("创建下载目录失败")?;
    }
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

    let initial_validator = response_validator(&response);
    let resumed = can_resume
        && response.status() == StatusCode::PARTIAL_CONTENT
        && initial_validator == request.validator;
    if can_resume && response.status() == StatusCode::PARTIAL_CONTENT && !resumed {
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

    let mut output = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&request.part_path)
        .context("打开部分归档失败")?;
    let started = Instant::now();
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
        let elapsed = started.elapsed().as_secs_f64();
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
    fn value(&self) -> &str {
        match self {
            Self::Etag(value) | Self::LastModified(value) => value,
        }
    }
}

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
    };

    use tiny_http::{Header, Response, Server, StatusCode};

    use super::{DownloadRequest, HttpValidator, Progress, download};

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
