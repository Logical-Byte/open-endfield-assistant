use std::{fs, sync::Arc};

use anyhow::Result;
use tauri::State;

use crate::{
    app_controller::{AppController, AppStatus},
    app_paths::AppPaths,
};

/// 启动档案库主任务（在后台线程执行，立即返回当前状态）。
#[tauri::command]
pub fn start_scan(state: State<'_, Arc<AppController>>) -> AppStatus {
    state.inner().start_scan();
    state.inner().get_status()
}

/// 请求停止主任务。
#[tauri::command]
pub fn stop_scan(state: State<'_, Arc<AppController>>) -> AppStatus {
    state.inner().stop_scan();
    state.inner().get_status()
}

/// 单次扫描当前档案详情。
#[tauri::command]
pub fn scan_single(state: State<'_, Arc<AppController>>) -> AppStatus {
    state.inner().scan_single();
    state.inner().get_status()
}

/// 查询当前应用状态。
#[tauri::command]
pub fn get_status(state: State<'_, Arc<AppController>>) -> AppStatus {
    state.inner().get_status()
}

/// 退出程序。
#[tauri::command]
pub fn quit(state: State<'_, Arc<AppController>>) {
    state.inner().quit();
}

/// 在系统文件管理器中打开日志目录（不存在时先创建）。
///
/// 由于根目录为
///   - 开发阶段：`src-tauri/` 的上一级（项目根）
///   - 打包阶段：exe 所在目录
/// 一个静态 scope 无法同时表达两种模式，想要在 capabilities 放通日志目录就必须写 `**/*`，这会带来安全风险。
/// 因此走 “前端 openPath + 精确 scope” 这条路是死的。
/// 我们选择用后端打开文件夹，而不是前端调用 `openPath`，这样可以不用在 capabilities 里放通 `**/*`。
/// Rust API open_path（后端直接调）完全不经过 scope 检查，路径由后端固定。
#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let logs_dir = AppPaths::new()
        .map_err(|e| format!("无法定位日志目录: {e}"))?
        .logs_dir();
    fs::create_dir_all(&logs_dir).map_err(|e| format!("无法创建日志目录: {e}"))?;
    tauri_plugin_opener::open_path(&logs_dir, None::<&str>)
        .map_err(|e| format!("无法打开日志目录: {e}"))
}
