//! 自动更新：下载、进度、取消与安装状态。

pub mod commands;
pub mod install;

mod download;
mod manager;
mod response;

pub use manager::UpdateManager;

use tracing::info;

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
