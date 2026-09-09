//! 更新包下载、进度上报、取消与 sha256 校验。

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;
use tracing::info;

use super::{UpdateManager, response::extract_filename_from_response};

/// 下载进度事件（前端按 `session_id` 过滤旧任务的迟到事件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    /// 下载会话编号（自增，用于区分新旧任务）
    pub session_id: u64,
    /// 已下载字节数
    pub downloaded_size: u64,
    /// 总字节数（未知时为 `0`）
    pub total_size: u64,
    /// EMA 平滑后的瞬时速度（字节/秒）
    pub speed: u64,
    /// 进度百分比（`0.0` ~ `100.0`，总大小未知时为 `0.0`）
    pub progress: f64,
}

/// 下载结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    /// 本次下载的会话编号
    pub session_id: u64,
    /// 实际保存路径（可能因 `Content-Disposition` / 重定向 URL 检测到真实文件名而不同于入参）
    pub actual_save_path: String,
    /// 检测到的文件名（未检测到时为 `None`）
    pub detected_filename: Option<String>,
}

/// 临时文件守卫：下载异常退出时同步删除 `.downloading` 半成品，成功重命名后调用 `disarm`。
struct TempFileGuard {
    path: Option<PathBuf>,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// 成功重命名后调用，使 drop 时不再尝试删除（文件已移至目标路径）。
    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            // 必须同步删除：异步删除可能在下一次下载创建同名临时文件后才执行，导致误删。
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// 进度上报任务守卫：在函数任意返回路径上都能停止采样循环。
struct ProgressEmitterGuard(Option<tokio::sync::oneshot::Sender<()>>);

impl Drop for ProgressEmitterGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

/// 按代理模式构建 HTTP 客户端。
///
/// - `none`：直连（显式 `no_proxy`）；
/// - `system`：解析系统代理后使用；
/// - `custom`：使用 `proxy_url`。
fn build_client(
    user_agent: &str,
    proxy_mode: Option<String>,
    proxy_url: Option<String>,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        // GitHub asset 端点会 302 到签名 CDN 地址，需要跟随重定向。
        .redirect(reqwest::redirect::Policy::limited(10));

    match proxy_mode.as_deref().unwrap_or("none") {
        "system" => {
            if let Some(url) = crate::platform::proxy::resolve_system_proxy()? {
                builder = builder.proxy(
                    reqwest::Proxy::all(url.as_str())
                        .map_err(|e| format!("系统代理配置失败: {e}"))?,
                );
            }
        }
        "custom" => {
            if let Some(url) = proxy_url.filter(|url| !url.trim().is_empty()) {
                builder = builder.proxy(
                    reqwest::Proxy::all(url.as_str()).map_err(|e| format!("代理配置失败: {e}"))?,
                );
            }
        }
        _ => {
            builder = builder.no_proxy();
        }
    }

    builder
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

/// 将字节切片编码为小写十六进制字符串。
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// 流式下载文件，支持进度事件、取消与 sha256 校验。
///
/// - 临时文件写入 `{save_path}.{session_id}.downloading`，成功后原子重命名；
/// - 校验失败 / 网络错误 / 取消时，`TempFileGuard` 负责删除半成品；
/// - 进度事件 `download-progress` 每 100ms 上报一次，带 `session_id` 供前端过滤；
/// - `accept` 由前端按下载源传入（GitHub 资产端点需要 `application/octet-stream`），
///   客户端自动跟随 302 重定向。
#[allow(clippy::too_many_arguments)] // 参数多但均为简单值，封装成结构体反而降低可读性
pub async fn download_update(
    manager: &UpdateManager,
    app: tauri::AppHandle,
    url: String,
    save_path: String,
    total_size: Option<u64>,
    expected_sha256: Option<String>,
    proxy_mode: Option<String>,
    proxy_url: Option<String>,
    accept: Option<String>,
    user_agent: Option<String>,
) -> Result<DownloadResult, String> {
    let download = manager
        .start_download()
        .map_err(|error| error.to_string())?;
    let session_id = download.id();
    let session = download.session();
    info!("download_update: session={session_id} url={url} -> {save_path}");

    let save_path_obj = Path::new(&save_path);
    if let Some(parent) = save_path_obj.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建下载目录: {e}"))?;
    }

    // UA 由前端基于 `@tauri-apps/plugin-os` 生成并传入（唯一来源）；
    // 兜底仅覆盖未传入 UA 的旧调用方。
    let user_agent = user_agent.unwrap_or_else(|| format!("OEA/{}", app.package_info().version));
    let client = build_client(&user_agent, proxy_mode, proxy_url)?;

    let mut request = client.get(&url);
    if let Some(accept) = accept.filter(|accept| !accept.trim().is_empty()) {
        request = request.header(reqwest::header::ACCEPT, accept.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let detected_filename = extract_filename_from_response(&response);
    let actual_save_path = if let Some(ref name) = detected_filename {
        save_path_obj
            .parent()
            .map(|parent| parent.join(name))
            .unwrap_or_else(|| PathBuf::from(name))
    } else {
        save_path_obj.to_path_buf()
    };

    let temp_path = format!("{}.{}.downloading", actual_save_path.display(), session_id);
    let mut temp_guard = TempFileGuard::new(PathBuf::from(&temp_path));

    let total = total_size
        .filter(|size| *size > 0)
        .or_else(|| response.content_length())
        .unwrap_or(0);

    // 有界通道将网络读取与磁盘写入解耦，避免大文件撑爆内存
    let (write_tx, write_rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(64);
    let temp_path_for_writer = temp_path.clone();
    let write_handle = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut write_rx = write_rx;
        let file = std::fs::File::create(&temp_path_for_writer)
            .map_err(|e| format!("无法创建临时文件: {e}"))?;
        let mut writer = std::io::BufWriter::with_capacity(512 * 1024, file);
        while let Some(chunk) = write_rx.blocking_recv() {
            writer
                .write_all(&chunk)
                .map_err(|e| format!("写入文件失败: {e}"))?;
        }
        writer
            .flush()
            .map_err(|e| format!("刷新写入缓冲区失败: {e}"))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|e| format!("同步文件失败: {e}"))?;
        Ok(())
    });

    // 独立进度上报任务，避免下载热路径被 emit 阻塞
    let app_for_emitter = app.clone();
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    let progress_guard = ProgressEmitterGuard(Some(stop_tx));
    let progress_session = Arc::clone(&session);
    tokio::spawn(async move {
        let mut last_downloaded = 0u64;
        let mut last_instant = tokio::time::Instant::now();
        let mut smoothed_speed: f64 = 0.0;
        const EMA_ALPHA: f64 = 0.3;
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    let downloaded = progress_session.downloaded_bytes();
                    let now = tokio::time::Instant::now();
                    let elapsed = now.duration_since(last_instant);
                    if elapsed.as_millis() == 0 {
                        continue;
                    }
                    let bytes_in_interval = downloaded.saturating_sub(last_downloaded);
                    let instant_speed = bytes_in_interval as f64 / elapsed.as_secs_f64();
                    smoothed_speed = if smoothed_speed == 0.0 {
                        instant_speed
                    } else {
                        EMA_ALPHA * instant_speed + (1.0 - EMA_ALPHA) * smoothed_speed
                    };
                    let progress = if total > 0 {
                        ((downloaded as f64 / total as f64) * 100.0).min(100.0)
                    } else {
                        0.0
                    };
                    let _ = app_for_emitter.emit(
                        "download-progress",
                        DownloadProgressEvent {
                            session_id,
                            downloaded_size: downloaded,
                            total_size: total,
                            speed: smoothed_speed as u64,
                            progress,
                        },
                    );
                    last_downloaded = downloaded;
                    last_instant = now;
                }
            }
        }
    });

    // 网络循环 + 流式 sha256（校验不额外增加 IO）
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut download_error: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        if session.is_cancelled() {
            download_error = Some("下载已取消".to_string());
            break;
        }
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                download_error = Some(format!("下载数据失败: {e}"));
                break;
            }
        };
        hasher.update(&chunk);
        let len = chunk.len() as u64;
        if write_tx.send(chunk).await.is_err() {
            download_error = Some("磁盘写入线程异常退出".to_string());
            break;
        }
        downloaded += len;
        session.set_downloaded_bytes(downloaded);
    }

    // 收尾前再检查一次取消标志
    if download_error.is_none() && session.is_cancelled() {
        download_error = Some("下载已取消".to_string());
    }

    // 关闭发送端，通知写入线程结束
    drop(write_tx);
    let write_result = write_handle
        .await
        .map_err(|e| format!("写入任务异常: {e}"))?;

    if let Some(error) = download_error {
        // 写入线程通常持有更具体的 I/O 错误（如磁盘满），优先返回
        if let Err(write_error) = write_result {
            return Err(write_error);
        }
        return Err(error);
    }
    write_result?;

    // sha256 校验（允许带 `sha256:` 前缀）
    let actual_hash = hex_encode(hasher.finalize().as_slice());
    if let Some(expected) = expected_sha256 {
        let expected = expected
            .trim()
            .trim_start_matches("sha256:")
            .to_ascii_lowercase();
        if !expected.is_empty() && actual_hash != expected {
            return Err(format!(
                "sha256 校验失败：期望 {expected}，实际 {actual_hash}"
            ));
        }
    }
    info!("sha256 校验通过: {actual_hash}");

    // 发送最终进度
    let _ = app.emit(
        "download-progress",
        DownloadProgressEvent {
            session_id,
            downloaded_size: downloaded,
            total_size: if total > 0 { total } else { downloaded },
            speed: 0,
            progress: 100.0,
        },
    );

    std::fs::rename(&temp_path, &actual_save_path)
        .map_err(|e| format!("重命名临时文件失败: {e}"))?;
    temp_guard.disarm();

    info!(
        "download_update 完成: {downloaded} 字节 -> {} (session {session_id})",
        actual_save_path.display()
    );
    drop(progress_guard);

    Ok(DownloadResult {
        session_id,
        actual_save_path: actual_save_path.to_string_lossy().into_owned(),
        detected_filename,
    })
}

/// 返回更新包下载目录（`<root>/cache/downloads`），不存在时创建。
pub fn get_update_download_dir() -> Result<String, String> {
    let dir = crate::app_paths::AppPaths::new()
        .map_err(|e| format!("无法定位应用根目录: {e}"))?
        .cache_dir()
        .join("downloads");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("无法创建下载目录 {}: {e}", dir.display()))?;
    Ok(dir.to_string_lossy().into_owned())
}

/// 解析 Windows 系统代理（前端检查请求与 Rust 下载共用）。
pub fn resolve_system_proxy() -> Result<Option<String>, String> {
    crate::platform::proxy::resolve_system_proxy()
}
