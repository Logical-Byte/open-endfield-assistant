//! Tauri 命令层：薄胶水，把前端 `invoke` 转发给 [`crate::controller::Controller`]。

use std::{fs, sync::Arc};

use anyhow::Result;
use tauri::State;

use crate::{
    app_paths::AppPaths,
    controller::{AppStatus, Controller},
};

/// 启动档案库主任务（在后台线程执行，立即返回当前状态）。
#[tauri::command]
pub fn start_scan(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().start_scan();
    state.inner().get_status()
}

/// 请求停止主任务。
#[tauri::command]
pub fn stop_scan(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().stop_scan();
    state.inner().get_status()
}

/// 单次扫描当前档案详情。
#[tauri::command]
pub fn scan_single(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().scan_single();
    state.inner().get_status()
}

/// 查询当前应用状态。
#[tauri::command]
pub fn get_status(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().get_status()
}

/// 退出程序。
#[tauri::command]
pub fn quit(state: State<'_, Arc<Controller>>) {
    state.inner().quit();
}

/// 在系统文件管理器中打开日志目录（不存在时先创建）。
///
/// 由于根目录为双模式动态路径（dev=项目根 / release=exe 目录），静态 scope 无法精确
/// 表达，故不用前端 `openPath` + scope 方案，而用后端 Rust API `open_path`
/// （直接调不经 scope 检查），capabilities 无需放通任何路径。
#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let logs_dir = AppPaths::new()
        .map_err(|e| format!("无法定位日志目录: {e}"))?
        .logs_dir();
    fs::create_dir_all(&logs_dir).map_err(|e| format!("无法创建日志目录: {e}"))?;
    tauri_plugin_opener::open_path(&logs_dir, None::<&str>)
        .map_err(|e| format!("无法打开日志目录: {e}"))
}
