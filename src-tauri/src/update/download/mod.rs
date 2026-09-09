//! 文件下载、进度上报、取消与 sha256 校验。

mod progress;
mod target;
mod transfer;

use std::{path::PathBuf, sync::Arc, time::Duration};

use tracing::info;

use progress::ProgressReporter;
use target::DownloadTarget;
use transfer::download_to_target;

use super::{
    UpdateManager,
    commands::{DownloadRequest, DownloadResult},
    response::extract_filename_from_response,
};

/// 按代理模式构建 HTTP 客户端。
///
/// - `none`：直连（显式 `no_proxy`）；
/// - `system`：解析系统代理后使用；
/// - `custom`：使用 `proxy_url`。
fn build_client(
    user_agent: &str,
    proxy_mode: Option<&str>,
    proxy_url: Option<&str>,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        // GitHub asset 端点会 302 到签名 CDN 地址，需要跟随重定向。
        .redirect(reqwest::redirect::Policy::limited(10));

    match proxy_mode.unwrap_or("none") {
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
                builder = builder
                    .proxy(reqwest::Proxy::all(url).map_err(|e| format!("代理配置失败: {e}"))?);
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

async fn open_response(
    app: &tauri::AppHandle,
    request: &DownloadRequest,
) -> Result<reqwest::Response, String> {
    let default_user_agent = format!("OEA/{}", app.package_info().version);
    let user_agent = request.user_agent.as_deref().unwrap_or(&default_user_agent);
    let client = build_client(
        user_agent,
        request.proxy_mode.as_deref(),
        request.proxy_url.as_deref(),
    )?;

    let mut http_request = client.get(&request.url);
    if let Some(accept) = request
        .accept
        .as_deref()
        .filter(|accept| !accept.trim().is_empty())
    {
        http_request = http_request.header(reqwest::header::ACCEPT, accept.trim());
    }

    let response = http_request
        .send()
        .await
        .map_err(|error| format!("下载请求失败: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }
    Ok(response)
}

fn resolve_save_path(
    requested_path: &str,
    response: &reqwest::Response,
) -> (PathBuf, Option<String>) {
    let requested_path = PathBuf::from(requested_path);
    let detected_filename = extract_filename_from_response(response);
    let actual_path = detected_filename.as_ref().map_or_else(
        || requested_path.clone(),
        |name| {
            requested_path
                .parent()
                .map(|parent| parent.join(name))
                .unwrap_or_else(|| PathBuf::from(name))
        },
    );
    (actual_path, detected_filename)
}

/// 流式下载文件，支持进度事件、取消与 sha256 校验。
///
/// - 临时文件写入 `{save_path}.{session_id}.downloading`，成功后原子重命名；
/// - 校验失败 / 网络错误 / 取消时，`DownloadTarget` 负责删除半成品；
/// - 进度事件 `download-progress` 每 100ms 上报一次，带 `session_id` 供前端过滤；
/// - `accept` 由前端按下载源传入（GitHub 资产端点需要 `application/octet-stream`），
///   客户端自动跟随 302 重定向。
pub(super) async fn run(
    manager: &UpdateManager,
    app: tauri::AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    let download = manager
        .start_download()
        .map_err(|error| error.to_string())?;
    let session_id = download.id();
    let session = download.session();
    info!(
        "download_file: session={session_id} url={} -> {}",
        request.url, request.save_path
    );

    let response = open_response(&app, &request).await?;
    let (actual_save_path, detected_filename) = resolve_save_path(&request.save_path, &response);
    let total = request
        .total_size
        .filter(|size| *size > 0)
        .or_else(|| response.content_length())
        .unwrap_or(0);
    let target = DownloadTarget::new(actual_save_path, session_id)?;
    let mut progress = ProgressReporter::start(app, session_id, Arc::clone(&session), total);
    let download = download_to_target(
        response,
        target,
        session,
        request.expected_sha256.as_deref(),
    )
    .await;
    progress.stop().await;
    let download = download?;

    info!("sha256 校验通过: {}", download.sha256);
    progress.emit_complete(download.downloaded_size, total);

    info!(
        "download_file 完成: {} 字节 -> {} (session {session_id})",
        download.downloaded_size,
        download.path.display()
    );

    Ok(DownloadResult {
        session_id,
        actual_save_path: download.path.to_string_lossy().into_owned(),
        detected_filename,
    })
}

/// 返回文件下载目录（`<root>/cache/downloads`），不存在时创建。
pub fn get_download_dir() -> Result<String, String> {
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
