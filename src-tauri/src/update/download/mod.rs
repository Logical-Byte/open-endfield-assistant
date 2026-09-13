//! 文件下载、进度上报、取消与 sha256 校验。

mod progress;
mod target;
mod transfer;

use std::{path::PathBuf, sync::Arc};

use tracing::info;

use crate::config::{OeaConfig, UpdateSource};

use progress::ProgressReporter;
use target::DownloadTarget;
use transfer::download_to_target;

use super::{
    commands::DownloadProgress,
    manager::DownloadSession,
    response::{extract_filename_from_response, sanitize_filename},
    source::DownloadPlan,
};

/// 执行后端解析完成的下载计划，并通过本次调用独享的 Channel 上报进度。
pub(super) async fn download_update_plan(
    plan: DownloadPlan,
    session_id: u64,
    session: Arc<DownloadSession>,
    config: &OeaConfig,
    user_agent: &str,
    on_progress: tauri::ipc::Channel<DownloadProgress>,
) -> Result<String, String> {
    info!(
        "download_update: session={session_id} source={:?} url={}",
        plan.source, plan.url
    );
    let client = match plan.source {
        UpdateSource::Mirrorchyan => super::http::build_direct_client(user_agent)?,
        UpdateSource::Oem | UpdateSource::Github => super::http::build_client(config, user_agent)?,
    };
    let mut request = client.get(&plan.url);
    if let Some(accept) = plan.accept {
        request = request.header(reqwest::header::ACCEPT, accept);
    }
    let cancellation = session.cancellation();
    let response = tokio::select! {
        biased;
        response = request.send() => response.map_err(|error| format!("下载请求失败: {error}"))?,
        _ = cancellation.cancelled() => return Err("下载已取消".to_string()),
    };
    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let download_dir = ensure_download_dir()?;
    let fallback_name = sanitize_filename(&plan.filename)
        .unwrap_or_else(|| "OEA-windows-x86_64-update.zip".to_string());
    let requested_path = download_dir.join(fallback_name);
    let detected_filename = extract_filename_from_response(&response);
    let actual_path =
        detected_filename.map_or_else(|| requested_path.clone(), |name| download_dir.join(name));
    let total = plan
        .total_size
        .filter(|size| *size > 0)
        .or_else(|| response.content_length())
        .unwrap_or(0);
    let target = DownloadTarget::new(actual_path, session_id)?;
    let mut progress = ProgressReporter::start_channel(on_progress, Arc::clone(&session), total);
    let download =
        download_to_target(response, target, session, plan.expected_sha256.as_deref()).await;
    progress.stop().await;
    let download = download?;

    info!("sha256 校验通过: {}", download.sha256);
    progress.emit_complete(download.downloaded_size, total);
    info!(
        "download_update 完成: {} 字节 -> {} (session {session_id})",
        download.downloaded_size,
        download.path.display()
    );
    Ok(download.path.to_string_lossy().into_owned())
}

/// 返回文件下载目录（`<root>/cache/downloads`），不存在时创建。
fn ensure_download_dir() -> Result<PathBuf, String> {
    let dir = crate::app_paths::AppPaths::new()
        .map_err(|e| format!("无法定位应用根目录: {e}"))?
        .cache_dir()
        .join("downloads");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("无法创建下载目录 {}: {e}", dir.display()))?;
    Ok(dir)
}
