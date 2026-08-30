//! Tauri 命令层：薄胶水，把前端 `invoke` 转发给 [`crate::controller::Controller`]。

use std::{fs, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD};
use tracing::{debug, error, info, trace, warn};

use crate::{
    app_paths::AppPaths,
    config::{self, OeaConfig},
    controller::{AppStatus, Controller},
    screenshot::{self, ScreenshotFormat},
    types::{ArchiveContract, PrtsData},
    windows_ops,
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

/// 返回 archive_contract.json 完整数据（前端用于按档案 id 查询获取方式）。
#[tauri::command]
pub fn get_archive_contract(state: tauri::State<Arc<Controller>>) -> Arc<ArchiveContract> {
    state.archive_contract_data()
}

/// 退出程序。
#[tauri::command]
pub fn quit(state: tauri::State<Arc<Controller>>) {
    state.quit();
}

/// 当前进程是否以管理员权限运行。
#[tauri::command]
pub fn is_elevated() -> bool {
    windows_ops::admin::is_elevated()
}

/// 以管理员权限重启应用（成功后退出当前进程）。
#[tauri::command]
pub fn restart_as_admin(app_handle: tauri::AppHandle) -> Result<(), String> {
    windows_ops::admin::restart_as_admin().map_err(|e| e.to_string())?;
    app_handle.exit(0);
    Ok(())
}

/// 读取 WebView2 当前缩放因子（`ZoomFactor`），用于前端初始化缩放滑块。
///
/// 缩放值的唯一持久化由 WebView2 自身负责（写入用户数据目录），
/// 前端只把它当作内存镜像，不再额外持久化。
#[tauri::command]
pub fn get_webview_zoom(window: tauri::WebviewWindow) -> Result<f64, String> {
    windows_ops::webview2::get_zoom(window).map_err(|e| e.to_string())
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
    debug!("正在保存配置 {oea_config:?} 到 {}", path.display());
    // 先保存到文件
    config::save_oea_config(&oea_config, &path).map_err(|e| {
        error!("保存配置文件失败: {e:#}");
        format!("{e:#}")
    })?;
    // 如果保存到文件成功，再更新内存配置，确保内存配置与磁盘配置一致
    let mut config_guard = state.oea_config().lock().unwrap_or_else(|e| e.into_inner());
    *config_guard = oea_config;
    info!("已成功保存配置到 {}", path.display());
    Ok(())
}

/// 用 DPAPI（当前用户作用域）加密 CDK，返回 Base64 密文。
#[tauri::command]
pub fn cdk_encrypt(cdk: String) -> Result<String, String> {
    let encrypted = windows_ops::dpapi::encrypt(cdk.trim().as_bytes()).map_err(|e| {
        error!("加密 CDK 失败: {e}");
        e.to_string()
    })?;
    Ok(STANDARD.encode(encrypted))
}

/// 用 DPAPI（当前用户作用域）解密 CDK 密文，返回明文。
#[tauri::command]
pub fn cdk_decrypt(encrypted: String) -> Result<String, String> {
    let blob = STANDARD.decode(encrypted.trim()).map_err(|e| {
        error!("CDK 密文 Base64 解码失败: {e}");
        e.to_string()
    })?;
    let plain = windows_ops::dpapi::decrypt(&blob).map_err(|e| {
        error!("解密 CDK 失败: {e}");
        e.to_string()
    })?;
    String::from_utf8(plain).map_err(|e| format!("CDK 明文不是合法 UTF-8: {e}"))
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

/// 写一条 TRACE 级日志到后端日志系统（进入文件 / 控制台，并广播给所有前端窗口）。
///
/// 前端通过这类命令把自己的运行信息接入统一日志管道，与后端日志一起排查问题。
#[tauri::command]
pub fn log_trace(message: String) {
    trace!("{}", message);
}

/// 写一条 DEBUG 级日志到后端日志系统（进入文件 / 控制台，并广播给所有前端窗口）。
#[tauri::command]
pub fn log_debug(message: String) {
    debug!("{}", message);
}

/// 写一条 INFO 级日志到后端日志系统（进入文件 / 控制台，并广播给所有前端窗口）。
#[tauri::command]
pub fn log_info(message: String) {
    info!("{}", message);
}

/// 写一条 WARN 级日志到后端日志系统（进入文件 / 控制台，并广播给所有前端窗口）。
#[tauri::command]
pub fn log_warn(message: String) {
    warn!("{}", message);
}

/// 写一条 ERROR 级日志到后端日志系统（进入文件 / 控制台，并广播给所有前端窗口）。
#[tauri::command]
pub fn log_error(message: String) {
    error!("{}", message);
}
