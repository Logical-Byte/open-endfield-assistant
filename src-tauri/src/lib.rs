//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

pub mod app_paths;
pub mod connect;
pub mod controller;
pub mod hotkey;
mod include;
pub mod input;
pub mod logger;
pub mod ocr;
pub mod resolution;
pub mod scene;
pub mod screencap;
pub mod session;
pub mod task;
pub mod tasks;
pub mod tauri_commands;
pub mod template_matching;
pub mod utils;
pub mod window;

use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex, mpsc};

use anyhow::{Result, anyhow};
use rapidocr_core::config::PipelineConfig;
use tauri::Manager;
use tracing::info;
use windows::Win32::Foundation::HWND;

use crate::{
    app_paths::AppPaths,
    controller::{Controller, HotkeyAction, HOTKEY_BINDINGS},
    hotkey::{HotkeyFilter, HotkeyRegistry},
    ocr::OcrEngine,
    scene::create_scene_manager,
    tasks::archive_scan::ScanResult,
    window::ForegroundGuard,
};

/// 获取 OEA 主窗口的原生窗口句柄（用于前台窗口判定）。
fn get_oea_hwnd(app: &tauri::App) -> Result<HWND> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| anyhow!("未找到 OEA 主窗口"))?;
    Ok(window.hwnd()?)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // WebView2 默认通过 raw input 接收键盘输入，当 OEA 窗口聚焦时会导致
        // WH_KEYBOARD_LL 低级键盘钩子收不到按键（tauri-apps/tauri#13919）。
        // Always = 移除 raw input 注册，让 LL 钩子全局都能收到按键。
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            tauri_commands::start_scan,
            tauri_commands::stop_scan,
            tauri_commands::scan_single,
            tauri_commands::get_status,
            tauri_commands::quit,
            tauri_commands::open_log_dir
        ])
        .setup(|app| {
            // 解析资源目录（resources/models/logs），不依赖运行时工作目录
            let app_paths = AppPaths::new()?;

            // 绿色便携：WebView2 用户数据目录放在应用目录内（默认会写入
            // %LOCALAPPDATA%\<identifier>），保证所有磁盘写入都限定在应用目录内。
            fs::create_dir_all(app_paths.webview_data_dir())?;
            let _main_window =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                    .title("OEA")
                    .inner_size(1024.0, 640.0)
                    .min_inner_size(256.0, 192.0)
                    .resizable(true)
                    .decorations(true)
                    .shadow(true)
                    .data_directory(app_paths.webview_data_dir())
                    .build()?;

            let (logger_guard, log_rx) = logger::init(&app_paths.logs_dir());
            window::set_thread_dpi_awareness_context();

            // 扫描结果通道：任务线程产生 → 转发线程 emit 给前端
            let (scan_tx, scan_rx) = mpsc::channel::<ScanResult>();
            let scan_index = Arc::new(AtomicU32::new(0));

            // 初始化 OCR 引擎（不依赖游戏窗口，任务开始时复用）
            let pipeline_config = PipelineConfig::recognition_only();
            let ocr_engine = OcrEngine::new(pipeline_config, &app_paths.models_dir())?;
            let ocr = Arc::new(Mutex::new(ocr_engine));

            // 场景管理器（本游戏全部场景，注册顺序即识别优先级）
            let scenes = Arc::new(create_scene_manager());

            // 全局热键：基础设施注册器 + 应用层键位 / 前台规则
            // 分号 / 引号仅在前台为 OEA 或终末地窗口时响应；Alt+Delete 退出全局生效
            let oea_hwnd = get_oea_hwnd(app)?;
            let foreground = ForegroundGuard::new(oea_hwnd);
            let filter: Option<HotkeyFilter> = Some(Box::new(move |tag| {
                if tag == HotkeyAction::Exit as u32 {
                    return true; // 退出全局生效
                }
                foreground.is_foreground_eligible()
            }));
            let hotkey = HotkeyRegistry::new(HOTKEY_BINDINGS, filter)?;

            // 状态标志（Controller 唯一归属）
            let stop = Arc::new(AtomicBool::new(false));
            let running = Arc::new(AtomicBool::new(false));

            // 组装 Controller 并托管为 State，启动后台线程
            let controller = Arc::new(Controller::new(
                ocr,
                app_paths.templates_dir(),
                scenes,
                hotkey,
                stop,
                running,
                scan_tx,
                scan_index,
                app.handle().clone(),
                logger_guard,
            ));
            Controller::spawn_log_loop(log_rx, app.handle().clone());
            Controller::spawn_scan_result_loop(scan_rx, app.handle().clone());
            Controller::spawn_hotkey_loop(&controller);
            app.manage(controller);

            info!("OEA 后端初始化完成");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            let (_, _) = (app_handle, event);
            // dbg!(app_handle, event);
        });
}
