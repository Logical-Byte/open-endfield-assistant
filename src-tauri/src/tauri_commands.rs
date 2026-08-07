//! Tauri 命令层：薄胶水，把前端 `invoke` 转发给 [`crate::controller::Controller`]。

use std::{fs, sync::Arc};

use tauri::State;

use crate::{
    app_paths::AppPaths,
    controller::{AppStatus, Controller},
    tasks::screenshot::{self, ScreenshotFormat},
    tray,
    types::PrtsData,
};

/// 启动扫描档案库任务（在后台线程执行，立即返回当前状态）。
#[tauri::command]
pub fn start_scan(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().start_scan();
    state.inner().get_status()
}

/// 请求停止扫描档案库任务。
#[tauri::command]
pub fn stop_scan(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().stop_scan();
    state.inner().get_status()
}

/// 查询当前应用状态。
#[tauri::command]
pub fn get_status(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().get_status()
}

/// 返回 prts.json 完整数据（前端用于分类中文名映射与自动补全候选）。
#[tauri::command]
pub fn get_prts_data(state: State<'_, Arc<Controller>>) -> Arc<PrtsData> {
    state.inner().prts_data()
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

/// 设置"关闭窗口时最小化到托盘"（返回设置后的值，供前端设置界面使用）。
#[tauri::command]
pub fn set_minimize_to_tray(enabled: bool) -> bool {
    tray::set_minimize_to_tray(enabled);
    tray::get_minimize_to_tray()
}

/// 查询"关闭窗口时最小化到托盘"。
#[tauri::command]
pub fn get_minimize_to_tray() -> bool {
    tray::get_minimize_to_tray()
}

/// 截取游戏窗口画面：按指定尺寸缩放并编码为指定格式（png / jpeg / webp），
/// 返回 base64 编码的图片数据（不含 data URL 前缀，由前端拼接）。
///
/// 本命令每次调用只执行一次截图；帧率控制、定时轮询等逻辑全部由前端负责。
#[tauri::command]
pub async fn screenshot(
    width: u32,
    height: u32,
    format: ScreenshotFormat,
) -> Result<String, String> {
    screenshot::capture_screenshot(width, height, format).map_err(|e| e.to_string())
}
