//! 文件下载、进度上报、取消与 sha256 校验。

mod progress;
mod target;
mod transfer;

use std::{path::PathBuf, sync::Arc};

use tracing::{debug, info, warn};

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
) -> Result<PathBuf, String> {
    let download_url = reqwest::Url::parse(&plan.url)
        .map_err(|url_error| format!("更新包下载地址无效: {url_error}"))?;
    let download_host = download_url.host_str().unwrap_or("<unknown-host>");
    let expected_sha256 = plan
        .expected_sha256
        .as_deref()
        .map(str::trim)
        .filter(|digest| !digest.is_empty());
    debug!(
        operation = "download",
        session_id,
        source = ?plan.source,
        host = %download_host,
        filename = %plan.filename,
        expected_bytes = ?plan.total_size,
        verify_sha256 = expected_sha256.is_some(),
        "更新包下载开始"
    );
    info!(
        "开始从 {} 下载更新包 {}",
        super::source::update_source_label(plan.source),
        plan.filename
    );
    let client = match plan.source {
        UpdateSource::Mirrorchyan => super::http::build_direct_client(user_agent)?,
        UpdateSource::Oem | UpdateSource::Github => super::http::build_client(config, user_agent)?,
    };
    let mut request = client.get(download_url);
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
    let download = download_to_target(response, target, session, expected_sha256).await;
    progress.stop().await;
    let download = download?;

    if expected_sha256.is_some() {
        debug!(
            operation = "download",
            session_id,
            sha256 = %download.sha256,
            "更新包 SHA-256 校验通过"
        );
        info!("更新包 {} 的完整性校验通过", plan.filename);
    } else {
        debug!(
            operation = "download",
            session_id,
            sha256 = %download.sha256,
            "更新包缺少预期 SHA-256"
        );
        warn!(
            "更新包 {} 未提供预期 SHA-256，已跳过完整性校验",
            plan.filename
        );
    }
    progress.emit_complete(download.downloaded_size, total);
    debug!(
        operation = "download",
        session_id,
        downloaded_bytes = download.downloaded_size,
        path = %download.path.display(),
        "更新包已原子发布到下载目录"
    );
    Ok(download.path)
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
