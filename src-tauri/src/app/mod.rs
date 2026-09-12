//! Tauri 应用外壳与生命周期编排。

mod commands;
mod hooks;
mod tray;

use std::fs;
use std::sync::{Arc, Mutex, mpsc};

use anyhow::{Context, Result};
use rapidocr_core::config::PipelineConfig;
use tauri::Manager;
use tracing::{info, warn};

use crate::{
    app_paths::AppPaths, config, controller::Controller, data::AppData, logger, ocr::OcrEngine,
    platform, scan_runtime::ScanRuntime, scene, scene::SceneManager, update,
};

use self::hooks::{crash, portable};

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
    // 启动时自动请求管理员权限（仅 `release`；用户取消则继续以普通权限运行）
    #[cfg(target_os = "windows")]
    platform::admin::elevate_at_startup();

    // 尽早安装全局 `panic hook`：任何 `panic`（含 Tauri `setup` 失败导致的 `panic`）都会
    // 独立写入 `logs/crash-*.log`，保证 `release`（无控制台）下也有可回溯记录。
    crash::install_panic_hook();

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        // `WebView2` 默认通过 `raw input` 接收键盘输入，当 OEA 窗口聚焦时会导致
        // `WH_KEYBOARD_LL` 低级键盘钩子收不到按键（[`tauri-apps/tauri#13919`](https://github.com/tauri-apps/tauri/issues/13919)）。
        // `Always` = 移除 `raw input` 注册，让 `LL` 钩子全局都能收到按键。
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_scan,
            commands::stop_scan,
            commands::get_status,
            commands::get_prts_data,
            commands::get_archive_contract,
            commands::quit,
            commands::open_log_dir,
            commands::load_oea_config,
            commands::save_oea_config,
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
            update::commands::download_file,
            update::commands::cancel_download,
            update::commands::get_download_dir,
            update::commands::resolve_system_proxy,
            update::install::install_update,
            update::install::consume_startup_update_result,
            update::install::pending_package_exists,
            update::install::extra::developer_choose_update_package,
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
            let (_, _) = (app_handle, event);
            // `dbg!(app_handle, event)`
        });
}

/// `setup` 主体：任何一步失败都会返回 `Err`，由 [`crash::report_fatal`] 统一兜底。
fn setup_app(app: &mut tauri::App) -> Result<()> {
    app.manage(update::UpdateManager::default());

    // 解析资源目录（`resources/models/logs`），不依赖运行时工作目录
    let app_paths = AppPaths::new()?;

    // 在初始化 Tauri 窗口、日志和资源消费者前完成 v2 的 resources 提交。helper 副本、
    // candidate 和 transaction 的生命周期都由 install core 管理，前端只消费结果。
    let workspace = update::install::UpdateWorkspace::for_current_executable(app_paths.root_dir())
        .map_err(|error| anyhow::anyhow!("无法确定更新 executable name: {error}"))?;
    let startup_update_result =
        update::install::complete_startup_transaction(&workspace).map_err(|error| {
            #[cfg(target_os = "macos")]
            platform::update::show_update_error("OEA 更新失败", &error);
            anyhow::anyhow!("启动时完成更新事务失败: {error}")
        })?;
    update::install::record_startup_update_result(startup_update_result);

    // 压缩包内直接运行检测：命中则弹原生框提示解压并退出。
    // 必须在建窗口 / 写 `cache` / 初始化日志之前调用（只读临时目录里这些步骤没有意义）。
    portable::ensure_extracted(&app_paths);

    // 初始化日志系统：控制台输出 DEBUG+，文件输出 TRACE+，前端转发 TRACE+（界面可过滤等级）。
    let (logger_guard, log_rx) = logger::init(&app_paths.logs_dir());

    // 设置线程 DPI 感知上下文，确保截图器获取的窗口客户区坐标与实际像素一致。
    platform::window::set_thread_dpi_awareness_context();

    // WebView2 缺失时自动下载引导程序并安装。
    platform::webview::ensure_installed(&app_paths.cache_dir()).inspect_err(|e| warn!("{e:#}"))?;

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
    platform::webview::register_zoom_changed_listener(&main_window);

    // 扫描结果通道：任务线程产生 → 转发线程 `emit` 给前端
    let (scan_tx, scan_rx) = mpsc::channel();

    // 初始化 OCR 引擎（不依赖游戏窗口，任务开始时复用）
    let pipeline_config = PipelineConfig::recognition_only();
    let ocr_engine = OcrEngine::new(pipeline_config, &app_paths.models_dir())?;
    let ocr = Arc::new(Mutex::new(ocr_engine));

    // 加载静态数据文件
    let app_data = AppData::load(&app_paths)?;

    // 场景管理器（本游戏全部场景，注册顺序即识别优先级）
    let scenes = Arc::new(SceneManager::new(vec![
        Box::new(scene::archive::Scene档案详情页面),
        Box::new(scene::archive::Scene档案库子界面),
        Box::new(scene::archive::Scene档案库主界面),
        Box::new(scene::terminal::Scene协议终端),
        Box::new(scene::overworld::Scene大世界),
        Box::new(scene::Scene未知),
    ]));

    // 开始监听热键
    let oea_window = platform::window::get_app_window(app.handle())?;
    let foreground = platform::window::ForegroundGuard::new(oea_window);
    let hotkey_rx = platform::hotkey::listen()?;

    let scan_runtime = Arc::new(ScanRuntime::new());

    // 组装 `Controller` 并托管为 `State`，启动后台线程
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

    // 初始化系统托盘（依赖已托管的 `Controller`，托盘菜单事件直接驱动它）
    tray::init_tray(app.handle())?;

    info!("OEA 后端初始化完成");
    Ok(())
}
