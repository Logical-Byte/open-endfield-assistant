//! Tauri 命令层：薄胶水，把前端 `invoke` 转发给 [`crate::controller::Controller`]。

use std::{fs, sync::Arc};

use tracing::{debug, error, info};

use crate::{
    admin,
    app_paths::AppPaths,
    config::{self, OeaConfig},
    controller::{AppStatus, Controller},
    tasks::screenshot::{self, ScreenshotFormat},
    types::{ArchiveAcquisitionContract, PrtsData},
};

/// 启动扫描档案库任务（在后台线程执行，立即返回当前状态）。
#[tauri::command]
pub fn start_scan(state: tauri::State<Arc<Controller>>) -> AppStatus {
    state.start_scan();
    state.get_status()
}

/// 请求停止扫描档案库任务。
#[tauri::command]
pub fn stop_scan(state: tauri::State<Arc<Controller>>) -> AppStatus {
    state.stop_scan();
    state.get_status()
}

/// 查询当前应用状态。
#[tauri::command]
pub fn get_status(state: tauri::State<Arc<Controller>>) -> AppStatus {
    state.get_status()
}

/// 返回 prts.json 完整数据（前端用于分类中文名映射与自动补全候选）。
#[tauri::command]
pub fn get_prts_data(state: tauri::State<Arc<Controller>>) -> Arc<PrtsData> {
    state.prts_data()
}

/// 返回 archive_acquisition_contract.json 完整数据（前端用于按档案 id 查询获取方式）。
#[tauri::command]
pub fn get_archive_acquisition_contract(
    state: tauri::State<Arc<Controller>>,
) -> Arc<ArchiveAcquisitionContract> {
    state.archive_acquisition_contract_data()
}

/// 退出程序。
#[tauri::command]
pub fn quit(state: tauri::State<Arc<Controller>>) {
    state.quit();
}

/// 当前进程是否以管理员权限运行。
#[tauri::command]
pub fn is_elevated() -> bool {
    admin::is_elevated()
}

/// 以管理员权限重启应用（成功后退出当前进程）。
#[tauri::command]
pub fn restart_as_admin(app_handle: tauri::AppHandle) -> Result<(), String> {
    admin::restart_as_admin().map_err(|e| e.to_string())?;
    app_handle.exit(0);
    Ok(())
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

/// 加载 OEA 配置文件。
#[tauri::command]
pub fn load_oea_config(state: tauri::State<Arc<Controller>>) -> OeaConfig {
    state.oea_config().lock().unwrap().clone()
}

/// 保存 OEA 配置文件。
#[tauri::command]
pub fn save_oea_config(
    state: tauri::State<Arc<Controller>>,
    oea_config: OeaConfig,
) -> Result<(), String> {
    let path = state.app_path().oea_config_file();
    debug!("正在保存配置 {oea_config:?} 到 {path:?}");
    // 先保存到文件
    config::save_oea_config(&oea_config, &path).map_err(|e| {
        error!("保存配置文件失败: {e:#}");
        format!("{e:#}")
    })?;
    // 如果保存到文件成功，再更新内存配置，确保内存配置与磁盘配置一致
    let mut config_guard = state.oea_config().lock().unwrap_or_else(|e| e.into_inner());
    *config_guard = oea_config;
    info!("已成功保存配置 {config_guard:?} 到 {path:?}");
    Ok(())
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
