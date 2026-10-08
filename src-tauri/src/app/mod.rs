//! Tauri 应用外壳与生命周期编排。

mod automation_events;
mod background_threads;
mod commands;
mod frontend_forwarders;
mod hooks;
mod hotkeys;
mod main_window;
mod tray;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use tauri::Manager;
use tracing::{info, warn};

use crate::{
    app_paths::AppPaths, automation, controller, data, logger, navigation, platform, settings,
    update, vision,
};

use self::hooks::{crash, portable};
use background_threads::BackgroundThreads;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 启动时自动请求管理员权限（仅 `release`；用户取消则继续以普通权限运行）
    #[cfg(target_os = "windows")]
    platform::admin::elevate_at_startup();

    // 尽早安装全局 `panic hook`：任何 `panic`（含 Tauri `setup` 失败导致的 `panic`）都会
    // 独立写入 `logs/crash-*.log`，保证 `release`（无控制台）下也有可回溯记录。
    crash::install_panic_hook();

    tauri::Builder::default()
        // `WebView2` 默认通过 `raw input` 接收键盘输入，当 OEA 窗口聚焦时会导致
        // `WH_KEYBOARD_LL` 低级键盘钩子收不到按键（[`tauri-apps/tauri#13919`](https://github.com/tauri-apps/tauri/issues/13919)）。
        // `Always` = 移除 `raw input` 注册，让 `LL` 钩子全局都能收到按键。
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_automation,
            commands::stop_automation,
            commands::get_automation_status,
            commands::get_prts_data,
            commands::get_archive_contract,
            commands::quit,
            commands::open_log_dir,
            commands::load_oea_settings,
            commands::save_oea_settings,
            commands::cdk_encrypt,
            commands::cdk_decrypt,
            commands::screenshot,
            commands::is_elevated,
            commands::restart_as_admin,
            commands::get_webview_zoom,
            commands::log_trace,
            commands::log_debug,
            commands::log_info,
            commands::log_warn,
            commands::log_error,
            update::commands::check_update,
            update::commands::get_update_status,
            update::commands::download_update,
            update::commands::cancel_download,
            update::install::install_update,
            update::install::consume_startup_update_result,
            update::install::extra::developer_install_update,
        ])
        .on_window_event(|window, event| {
            // 关闭窗口时：若启用最小化到托盘，则隐藏窗口而不是退出应用
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app_handle = window.app_handle();
                // 安装更新期间拒绝关闭窗口（配合不可关闭的安装弹窗）
                if app_handle.state::<update::UpdateManager>().is_installing() {
                    warn!("正在安装更新，拒绝关闭窗口");
                    api.prevent_close();
                    return;
                }
                let controller = app_handle.state::<controller::Controller>();
                if controller.settings_snapshot().minimize_to_tray {
                    api.prevent_close();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }
        })
        .setup(|app| {
            // `setup` 失败不允许向上传播：Tauri 会直接 `panic`（`Failed to setup app`）且
            // `release` 无控制台，用户毫无感知。统一交给 `crash::report_fatal` 兜底：
            // 全链日志 + `crash` 文件 + 原生弹窗 + 退出。
            if let Err(e) = setup_app(app) {
                crash::report_fatal(&e, app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(background_threads) = app_handle.try_state::<BackgroundThreads>() {
                    background_threads.shutdown();
                }
            }
        });
}

/// `setup` 主体：任何一步失败都会返回 `Err`，由 [`crash::report_fatal`] 统一兜底。
fn setup_app(app: &mut tauri::App) -> Result<()> {
    // 解析资源目录（`resources/models/logs`），不依赖运行时工作目录
    let app_paths = AppPaths::new().map_err(anyhow::Error::msg)?;

    // 检测是否未解压。必须在一切剩余的副作用操作前进行。
    portable::ensure_extracted(&app_paths);

    // 更新事务可能在正常应用初始化前失败，因此先启用文件日志。
    // 前端会在 BackgroundThreads 启动转发线程后收到在这期间缓存的日志。
    let (logger_guard, log_rx) = logger::init(&app_paths.logs_dir());

    // 必须在应用初始化前完成更新。
    update::install::initialize_at_startup(&app_paths)?;

    // 开始应用初始化。

    // 设置线程 DPI 感知上下文，确保截图器获取的窗口客户区坐标与实际像素一致。
    platform::window::set_thread_dpi_awareness_context();

    // 窗口关闭处理和前端更新命令需要此状态，必须在创建窗口之前托管。
    app.manage(update::UpdateManager::default());

    // 创建 Webview 窗口。
    main_window::create(app, &app_paths)?;

    // 加载后端资源。
    let settings_store = settings::SettingsStore::at(app_paths.oea_settings_file());
    let ocr = vision::ocr::OcrEngine::new(&app_paths.models_dir(), vision::ocr::Config::default())?;
    let app_data = data::AppData::load(&app_paths)?;
    let navigator = navigation::Navigator::new();
    let frontend_event_sink = Arc::new(automation_events::TauriEventSink::start(
        app.handle().clone(),
    )?);
    let automation_runtime = automation::Runtime::new(Arc::clone(&frontend_event_sink));
    let controller = controller::Controller::new(
        Arc::new(settings_store),
        Arc::new(Mutex::new(ocr)),
        Arc::new(navigator),
        Arc::new(automation_runtime),
        app_data,
    );
    app.manage(controller);

    // 初始化系统托盘，依赖已托管的 `Controller`。
    tray::init_tray(app.handle())?;

    // 统一管理后台线程。
    let background_threads =
        BackgroundThreads::start(app.handle(), frontend_event_sink, logger_guard, log_rx)?;
    app.manage(background_threads);

    info!("OEA 后端初始化完成");
    Ok(())
}
