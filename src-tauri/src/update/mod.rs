//! 自动更新：下载、进度、取消与安装状态。

pub mod install;

mod download;
mod manager;
mod response;

pub use download::{DownloadProgressEvent, DownloadResult};
pub use manager::UpdateManager;

use tracing::info;

/// Stream an update package to disk while preserving the existing command surface.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn download_update(
    manager: tauri::State<'_, UpdateManager>,
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
        &manager,
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
pub fn cancel_download(manager: tauri::State<'_, UpdateManager>) -> Result<(), String> {
    manager.cancel_download().map_err(|error| error.to_string())
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

/// 驱动更新安装状态迁移。
///
/// `installing = true` 仅允许从 `Idle` 进入 `Installing`；`false` 仅允许从
/// `Installing` 返回 `Idle`。其他状态迁移会返回错误。
#[tauri::command]
pub fn set_update_installing(
    manager: tauri::State<'_, UpdateManager>,
    installing: bool,
) -> Result<(), String> {
    let result = if installing {
        manager.begin_install()
    } else {
        manager.finish_install()
    };
    result.map_err(|error| error.to_string())?;
    info!("设置更新安装状态: {installing}");
    Ok(())
}
