//! 自动更新：下载、进度、取消与安装状态。

pub mod install;

mod download;
mod response;

pub use download::{DownloadProgressEvent, DownloadResult};

use std::sync::atomic::{AtomicBool, Ordering};

use tracing::info;

/// Stream an update package to disk while preserving the existing command surface.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn download_update(
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
    download::download_update(
        app,
        url,
        save_path,
        total_size,
        expected_sha256,
        proxy_mode,
        proxy_url,
        accept,
        user_agent,
    )
    .await
}

/// Cancel the current update download.
#[tauri::command]
pub fn cancel_download() {
    download::cancel_download();
}

/// Return the update package download directory.
#[tauri::command]
pub fn get_update_download_dir() -> Result<String, String> {
    download::get_update_download_dir()
}

/// Resolve the Windows system proxy used by update checks and downloads.
#[tauri::command]
pub fn resolve_system_proxy() -> Result<Option<String>, String> {
    download::resolve_system_proxy()
}

/// 安装进行中标志：安装期间拒绝退出（`quit` / 窗口关闭 / 托盘退出统一检查）。
static UPDATE_INSTALLING: AtomicBool = AtomicBool::new(false);

/// 当前是否正在安装更新。
pub fn is_installing() -> bool {
    UPDATE_INSTALLING.load(Ordering::SeqCst)
}

/// 设置安装进行中标志（前端在安装开始/结束时调用）。
#[tauri::command]
pub fn set_update_installing(installing: bool) {
    UPDATE_INSTALLING.store(installing, Ordering::SeqCst);
    info!("设置更新安装状态: {installing}");
}
