//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

pub mod app_paths;
pub mod config;
pub mod connect;
pub mod controller;
pub mod crash;
pub mod data;
pub mod dpapi;
pub mod logger;
pub mod ocr;
pub mod portable;
pub mod resolution;
pub(crate) mod scan_runtime;
pub mod scene;
pub mod session;
pub mod sound;
pub mod task;
pub mod tasks;
pub mod tauri_commands;
pub mod template_matching;
pub mod tray;
pub mod types;
pub mod update;
pub mod utils;
pub mod windows_ops;

use std::fs;
use std::sync::{Arc, Mutex, mpsc};

use anyhow::{Context, Result};
use rapidocr_core::config::PipelineConfig;
use tauri::Manager;
use tracing::{info, warn};

use crate::{
    app_paths::AppPaths, controller::Controller, data::AppData, ocr::OcrEngine,
    scan_runtime::ScanRuntime, scene::create_scene_manager,
};

#[cfg(target_os = "windows")]
fn configure_main_window<'a, R, M>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
    app_paths: &AppPaths,
) -> tauri::WebviewWindowBuilder<'a, R, M>
where
    R: tauri::Runtime,
    M: tauri::Manager<R>,
{
    builder.data_directory(app_paths.webview_data_dir())
}

#[cfg(target_os = "macos")]
fn configure_main_window<'a, R, M>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
    _app_paths: &AppPaths,
) -> tauri::WebviewWindowBuilder<'a, R, M>
where
    R: tauri::Runtime,
    M: tauri::Manager<R>,
{
    builder.incognito(true)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        // WebView2 默认通过 raw input 接收键盘输入，当 OEA 窗口聚焦时会导致
        // WH_KEYBOARD_LL 低级键盘钩子收不到按键（[`tauri-apps/tauri#13919`](https://github.com/tauri-apps/tauri/issues/13919)）。
        // Always = 移除 raw input 注册，让 LL 钩子全局都能收到按键。
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            tauri_commands::start_scan,
            tauri_commands::stop_scan,
            tauri_commands::get_status,
            tauri_commands::get_prts_data,
            tauri_commands::get_archive_contract,
            tauri_commands::quit,
            tauri_commands::open_log_dir,
            tauri_commands::load_oea_config,
            tauri_commands::save_oea_config,
            tauri_commands::cdk_encrypt,
            tauri_commands::cdk_decrypt,
            tauri_commands::screenshot,
            tauri_commands::is_elevated,
            tauri_commands::restart_as_admin,
            tauri_commands::get_webview_zoom,
            tauri_commands::log_trace,
            tauri_commands::log_debug,
            tauri_commands::log_info,
            tauri_commands::log_warn,
            tauri_commands::log_error,
            update::download_update,
            update::cancel_download,
            update::get_update_download_dir,
            update::resolve_system_proxy,
            update::set_update_installing,
            update::install::backup_config,
            update::install::extract_zip,
            update::install::check_changes_json,
            update::install::apply_incremental_update,
            update::install::apply_full_update,
            update::install::restore_from_old,
            update::install::cleanup_old_dir,
            update::install::cleanup_extract_dir,
            update::install::remove_downloaded_package,
            update::install::pending_package_exists,
            update::install::cleanup_stale_update_files,
        ])
        .on_window_event(|window, event| {
            // 关闭窗口时：若启用最小化到托盘，则隐藏窗口而不是退出应用
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 安装更新期间拒绝关闭窗口（配合不可关闭的安装弹窗）
                if update::is_installing() {
                    warn!("正在安装更新，拒绝关闭窗口");
                    api.prevent_close();
                    return;
                }
                let app_handle = window.app_handle();
                let controller = app_handle.state::<Arc<Controller>>();
                if controller
                    .oea_config()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .minimize_to_tray
                {
                    api.prevent_close();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }
        })
        .setup(|app| {
            // setup 失败不允许向上传播：Tauri 会直接 panic（`Failed to setup app`）且
            // release 无控制台，用户毫无感知。统一交给 crash::report_fatal 兜底：
            // 全链日志 + crash 文件 + 原生弹窗 + 退出。
            if let Err(e) = setup_app(app) {
                crash::report_fatal(&e, app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            let (_, _) = (app_handle, event);
            // dbg!(app_handle, event);
        });
}

/// setup 主体：任何一步失败都会返回 Err，由 [`crash::report_fatal`] 统一兜底。
fn setup_app(app: &mut tauri::App) -> Result<()> {
    // 解析资源目录（resources/models/logs），不依赖运行时工作目录
    let app_paths = AppPaths::new()?;

    // 压缩包内直接运行检测：命中则弹原生框提示解压并退出。
    // 必须在建窗口 / 写 cache / 初始化日志之前调用（只读临时目录里这些步骤没有意义）。
    portable::ensure_extracted(&app_paths);

    // 初始化日志系统：控制台输出 DEBUG+，文件输出 TRACE+，前端转发 TRACE+（界面可过滤等级）。
    let (logger_guard, log_rx) = logger::init(&app_paths.logs_dir());

    // 设置线程 DPI 感知上下文，确保截图器获取的窗口客户区坐标与实际像素一致。
    windows_ops::window::set_thread_dpi_awareness_context();

    // WebView2 缺失时自动下载引导程序并安装。
    windows_ops::webview2::ensure_installed(&app_paths.cache_dir())
        .inspect_err(|e| warn!("{e:#}"))?;

    // 解析应用配置文件
    let oea_config = Arc::new(Mutex::new(config::load_oea_config(
        &app_paths.oea_config_file(),
    )));

    // 绿色便携：WebView2 用户数据目录放在应用目录内（默认会写入 `%LOCALAPPDATA%\<identifier>`），保证所有磁盘写入都限定在应用目录内。
    fs::create_dir_all(app_paths.webview_data_dir()).with_context(|| {
        format!(
            "创建 WebView2 数据目录 {} 失败",
            app_paths.webview_data_dir().display()
        )
    })?;
    // 在 Rust 里动态创建 webview 窗口，而不在 `tauri.conf.json` 里声明窗口，否则无法更改 WebView2 用户数据目录。
    let main_window_builder =
        tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
            .title("OEA")
            .inner_size(1024.0, 640.0)
            .min_inner_size(864.0, 540.0)
            .resizable(true)
            .decorations(false) // 移除系统标题栏
            .shadow(true)
            .data_directory(app_paths.webview_data_dir())
            .zoom_hotkeys_enabled(true); // 允许 Ctrl+滚轮 / Ctrl++ / Ctrl+- 原生缩放

    let main_window_builder = configure_main_window(main_window_builder, &app_paths);

    let main_window = main_window_builder.build()?;
    register_zoom_changed_listener(&main_window);

    // 扫描结果通道：任务线程产生 → 转发线程 emit 给前端
    let (scan_tx, scan_rx) = mpsc::channel();

    // 初始化 OCR 引擎（不依赖游戏窗口，任务开始时复用）
    let pipeline_config = PipelineConfig::recognition_only();
    let ocr_engine = OcrEngine::new(pipeline_config, &app_paths.models_dir())?;
    let ocr = Arc::new(Mutex::new(ocr_engine));

    // 加载静态数据文件
    let app_data = AppData::load(&app_paths)?;

    // 场景管理器（本游戏全部场景，注册顺序即识别优先级）
    let scenes = Arc::new(create_scene_manager());

    // 开始监听热键
    let oea_window = windows_ops::window::get_app_window(app.handle())?;
    let foreground = windows_ops::window::ForegroundGuard::new(oea_window);
    let hotkey_rx = windows_ops::hotkey::listen()?;

    let scan_runtime = Arc::new(ScanRuntime::new());

    // 组装 Controller 并托管为 State，启动后台线程
    let controller = Arc::new(Controller::new(
        app_paths,
        oea_config,
        ocr,
        scenes,
        scan_runtime,
        scan_tx,
        foreground,
        app.handle().clone(),
        app_data,
        logger_guard,
    ));
    Controller::spawn_log_loop(log_rx, app.handle().clone());
    Controller::spawn_scan_result_loop(scan_rx, app.handle().clone());
    controller.spawn_hotkey_loop(hotkey_rx);
    app.manage(controller);

    // 初始化系统托盘（依赖已托管的 Controller，托盘菜单事件直接驱动它）
    tray::init_tray(app.handle())?;

    info!("OEA 后端初始化完成");
    Ok(())
}

/// 注册 WebView2 原生缩放（`ZoomFactor`）变化监听。
///
/// 用户通过 `Ctrl+滚轮` / `Ctrl+加减` 缩放时，WebView2 内部会修改 `ZoomFactor` 并触发
/// `ZoomFactorChanged` 事件。这里把新值 emit 给前端（`webview-zoom-changed`），
/// 让设置页的缩放滑块与快捷键缩放保持同步。
fn register_zoom_changed_listener(window: &tauri::WebviewWindow) {
    use tauri::Emitter;
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::ICoreWebView2Controller, ZoomFactorChangedEventHandler,
    };
    use windows::core::IUnknown;

    let app_handle = window.app_handle().clone();

    let result = window.with_webview(move |platform_webview| {
        let controller = platform_webview.controller();

        let handler = ZoomFactorChangedEventHandler::create(Box::new(
            move |sender: Option<ICoreWebView2Controller>, _args: Option<IUnknown>| {
                let Some(controller) = sender else {
                    return Ok(());
                };
                let mut factor = 0.0f64;
                unsafe { controller.ZoomFactor(&mut factor) }?;
                let _ = app_handle.emit("webview-zoom-changed", factor);
                Ok(())
            },
        ));

        let mut token = 0i64;
        if let Err(e) = unsafe { controller.add_ZoomFactorChanged(&handler, &mut token) } {
            warn!("注册 ZoomFactorChanged 监听失败: {e}");
        }
    });

    if let Err(e) = result {
        warn!("获取 WebView2 controller 失败: {e:#}");
    }
}
