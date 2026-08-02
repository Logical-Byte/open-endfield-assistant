//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

// ============ 业务模块（原 dak 逻辑） ============
pub mod app;
pub mod app_controller;
pub mod app_paths;
pub mod geometry;
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
pub mod template_matching;
pub mod utils;
pub mod window;

use std::sync::Arc;

use rapidocr_core::config::PipelineConfig;
use tauri::{Manager, State};
use tracing::info;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_NOREPEAT, VK_DELETE, VK_OEM_1, VK_OEM_7,
};

use crate::{
    app::App,
    app_controller::AppController,
    app_paths::AppPaths,
    hotkey::{HotkeyBinding, HotkeyEvent, HotkeyListener},
    input::{InputBase, SeizeInput},
    ocr::OcrEngine,
    resolution::GameResolution,
    screencap::PrintWindowScreencap,
    session::Session,
    tasks::archive_scan::scenes::create_scene_manager,
};

// ============ Tauri 命令 ============

/// 启动档案库主任务（在后台线程执行，立即返回当前状态）。
#[tauri::command]
fn start_scan(state: State<'_, Arc<AppController>>) -> app_controller::AppStatus {
    state.inner().start_scan();
    state.inner().get_status()
}

/// 请求停止主任务。
#[tauri::command]
fn stop_scan(state: State<'_, Arc<AppController>>) -> app_controller::AppStatus {
    state.inner().stop_scan();
    state.inner().get_status()
}

/// 单次扫描当前档案详情。
#[tauri::command]
fn scan_single(state: State<'_, Arc<AppController>>) -> app_controller::AppStatus {
    state.inner().scan_single();
    state.inner().get_status()
}

/// 查询当前应用状态。
#[tauri::command]
fn get_status(state: State<'_, Arc<AppController>>) -> app_controller::AppStatus {
    state.inner().get_status()
}

/// 退出程序。
#[tauri::command]
fn quit(state: State<'_, Arc<AppController>>) {
    state.inner().quit();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            start_scan,
            stop_scan,
            scan_single,
            get_status,
            quit
        ])
        .setup(|app| {
            // 解析资源目录（resources/models/logs），不依赖运行时工作目录
            let paths = AppPaths::new()?;
            let (logger_guard, log_rx) = logger::init(&paths.logs_dir);

            window::set_thread_dpi_awareness_context();

            // 1. 获取游戏窗口（仅确保窗口在屏幕上，不抢占前台）
            let hwnd = window::get_window_by_title("Endfield", Some("UnityWndClass"))?;
            window::ensure_window_on_screen(hwnd)?;

            let client_rect = window::get_client_rect(hwnd)?;
            let resolution =
                GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
            info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

            // 2. 初始化底层组件（截图、输入、OCR）
            let screencap = Box::new(PrintWindowScreencap::new(hwnd));
            let input = Box::new(SeizeInput::new(hwnd, false));
            let pipeline_config = PipelineConfig::recognition_only();
            let ocr_engine = OcrEngine::new(pipeline_config, &paths.models_dir)?;

            // 3. 注册全局热键
            let hotkey = HotkeyListener::new(&[
                HotkeyBinding {
                    vk: VK_OEM_1.0 as u32,
                    modifiers: HOT_KEY_MODIFIERS(0),
                    event: HotkeyEvent::ScanSingleArchive,
                },
                HotkeyBinding {
                    vk: VK_OEM_7.0 as u32,
                    modifiers: HOT_KEY_MODIFIERS(0),
                    event: HotkeyEvent::ToggleMainTask,
                },
                HotkeyBinding {
                    vk: VK_DELETE.0 as u32,
                    modifiers: MOD_ALT | MOD_NOREPEAT,
                    event: HotkeyEvent::ExitProgram,
                },
            ])?;
            let stop_flag = hotkey.stop_flag();
            let running = hotkey.main_running_flag();

            // 4. 创建 Session（传入停止标志，识别 / 输入操作前会检查）
            let session = Session::new(
                hwnd,
                screencap,
                input,
                ocr_engine,
                paths.templates_dir(),
                resolution,
                stop_flag.clone(),
            );

            // 5. 构建场景管理器与应用
            let scene_manager = create_scene_manager();
            let app_core = App::new(session, scene_manager, running.clone());

            // 6. 组装 AppController 并托管为 State，启动后台热键轮询线程
            let controller = Arc::new(AppController::new(
                app_core,
                hotkey,
                stop_flag,
                running,
                app.handle().clone(),
                logger_guard,
            ));
            AppController::spawn_log_loop(log_rx, app.handle().clone());
            AppController::spawn_hotkey_loop(&controller);
            app.manage(controller);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
