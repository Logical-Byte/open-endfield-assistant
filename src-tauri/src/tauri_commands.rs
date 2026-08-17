//! Tauri 命令层：薄胶水，把前端 `invoke` 转发给 [`crate::controller::Controller`]。

use std::{fs, sync::Arc};

use tauri::Emitter;
use tracing::{debug, error, info};

use crate::{
    app_paths::AppPaths,
    config::{self, OeaConfig},
    controller::{AppStatus, Controller},
    tasks::screenshot::{self, ScreenshotFormat},
    types::{ArchiveAcquisitionContract, PrtsData},
    update::workflow::{UPDATE_EVENT, UpdateManager, UpdateSnapshot},
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
    windows_ops::admin::is_elevated()
}

/// 以管理员权限重启应用（成功后退出当前进程）。
#[tauri::command]
pub fn restart_as_admin(app_handle: tauri::AppHandle) -> Result<(), String> {
    windows_ops::admin::restart_as_admin().map_err(|e| e.to_string())?;
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

/// 返回 Rust 更新管理器的完整快照，供前端首次加载或事件丢失后重新同步。
#[tauri::command]
pub fn update_get_snapshot(state: tauri::State<Arc<UpdateManager>>) -> UpdateSnapshot {
    state.snapshot()
}

/// 在阻塞线程检查更新，并把每个完整状态快照作为 Tauri 事件发给前端。
///
/// `reqwest` 使用阻塞 API；`spawn_blocking` 避免网络请求占住 Tauri 的异步执行线程。
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    updater: tauri::State<'_, Arc<UpdateManager>>,
    controller: tauri::State<'_, Arc<Controller>>,
) -> Result<(), String> {
    let updater = Arc::clone(updater.inner());
    let config = controller
        .oea_config()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        updater.check(&config, |snapshot| {
            let _ = app.emit(UPDATE_EVENT, snapshot);
        })
    })
    .await
    .map_err(|error| format!("检查更新任务失败: {error}"))?
    .map_err(|error| format!("{error:#}"))
}

#[cfg(target_os = "windows")]
/// 下载并准备完整包，然后启动独立 Bootstrap 完成 Windows 入口替换。
///
/// Rust 工作线程拥有下载、校验和磁盘事务。准备成功后主程序只负责启动 Bootstrap，
/// 再退出自身以释放 `OEA.exe`；前端不会编排这些文件系统操作。
#[tauri::command]
pub async fn update_download_and_install(
    app: tauri::AppHandle,
    updater: tauri::State<'_, Arc<UpdateManager>>,
    controller: tauri::State<'_, Arc<Controller>>,
) -> Result<(), String> {
    let updater = Arc::clone(updater.inner());
    let config = controller
        .oea_config()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let current_exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let caller_pid = std::process::id();
    let worker_app = app.clone();
    let worker_updater = Arc::clone(&updater);
    let handoff = tauri::async_runtime::spawn_blocking(move || {
        worker_updater.download_and_prepare(&config, &current_exe, caller_pid, |snapshot| {
            let _ = worker_app.emit(UPDATE_EVENT, snapshot);
        })
    })
    .await
    .map_err(|error| format!("安装更新任务失败: {error}"))?
    .map_err(|error| format!("{error:#}"))?;

    // Bootstrap 必须是独立进程，才能等当前 OEA 退出后替换仍被 Windows 锁定的入口。
    if let Err(error) = std::process::Command::new(&handoff.bootstrap_path)
        .arg("--bootstrap-update")
        .arg(&handoff.portable_root)
        .arg(&handoff.transaction_dir)
        .spawn()
    {
        let message = format!("启动 Bootstrap 失败: {error}");
        updater.fail(message.clone(), |snapshot| {
            let _ = app.emit(UPDATE_EVENT, snapshot);
        });
        return Err(message);
    }
    app.exit(0);
    Ok(())
}

#[cfg(target_os = "macos")]
/// macOS 开发外壳只支持前端和可移植逻辑调试，不安装 Windows 便携包。
#[tauri::command]
pub async fn update_download_and_install() -> Result<(), String> {
    Err("自动安装仅支持 Windows 绿色便携版".into())
}
