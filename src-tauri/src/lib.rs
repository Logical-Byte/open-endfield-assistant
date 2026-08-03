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
pub mod scan_result;
pub mod scene;
pub mod screencap;
pub mod session;
pub mod session_factory;
pub mod task;
pub mod tasks;
pub mod template_matching;
pub mod utils;
pub mod window;

use std::{
    ffi::c_void,
    sync::{Arc, Mutex, mpsc},
};

use anyhow::{Result, anyhow, bail};
use rapidocr_core::config::PipelineConfig;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::{Manager, State};
use windows::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, MOD_ALT, MOD_NOREPEAT, VK_DELETE, VK_OEM_1, VK_OEM_7,
    },
};

use crate::{
    app::App,
    app_controller::AppController,
    app_paths::AppPaths,
    hotkey::{HotkeyBinding, HotkeyEvent, HotkeyListener},
    ocr::OcrEngine,
    scan_result::ScanResult,
    session_factory::SessionFactory,
    tasks::archive_scan::scenes::create_scene_manager,
    window::ForegroundGuard,
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

/// 获取 OEA 主窗口的原生窗口句柄（用于前台窗口判定）。
fn get_oea_hwnd(app: &tauri::App) -> Result<HWND> {
    let webview = app
        .get_webview_window("main")
        .ok_or_else(|| anyhow!("未找到 OEA 主窗口"))?;
    let raw = webview.window_handle()?.as_raw();
    match raw {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut c_void)),
        _ => bail!("非 Windows 平台，无法获取 OEA 窗口句柄"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            // 0. 扫描结果通道：任务线程产生 → 转发线程 emit 给前端
            let (scan_result_tx, scan_result_rx) = mpsc::channel::<ScanResult>();

            // 1. 初始化 OCR 引擎（不依赖游戏窗口，任务开始时复用）
            let pipeline_config = PipelineConfig::recognition_only();
            let ocr_engine = OcrEngine::new(pipeline_config, &paths.models_dir)?;
            let ocr = Arc::new(Mutex::new(ocr_engine));

            // 2. 注册全局热键
            // 分号/引号仅在前台为 OEA 或终末地窗口时响应；Alt+Delete 退出全局生效
            let foreground = ForegroundGuard::new(get_oea_hwnd(app)?);
            let hotkey = HotkeyListener::new(
                &[
                    HotkeyBinding {
                        vk: VK_OEM_1.0 as u32,
                        modifiers: HOT_KEY_MODIFIERS::default(),
                        event: HotkeyEvent::ScanSingleArchive,
                    },
                    HotkeyBinding {
                        vk: VK_OEM_7.0 as u32,
                        modifiers: HOT_KEY_MODIFIERS::default(),
                        event: HotkeyEvent::ToggleMainTask,
                    },
                    HotkeyBinding {
                        vk: VK_DELETE.0 as u32,
                        modifiers: MOD_ALT | MOD_NOREPEAT,
                        event: HotkeyEvent::ExitProgram,
                    },
                ],
                foreground,
            )?;
            let stop_flag = hotkey.stop_flag();
            let running = hotkey.main_running_flag();

            // 3. 创建会话工厂（窗口与分辨率在任务开始时才检查，避免游戏未打开时启动失败）
            let session_factory = SessionFactory::new(
                ocr,
                paths.templates_dir(),
                stop_flag.clone(),
                scan_result_tx.clone(),
            );

            // 4. 构建场景管理器与应用
            let scene_manager = create_scene_manager();
            let app_core = App::new(session_factory, scene_manager, running.clone());

            // 5. 组装 AppController 并托管为 State，启动后台热键轮询线程
            let controller = Arc::new(AppController::new(
                app_core,
                hotkey,
                stop_flag,
                running,
                app.handle().clone(),
                logger_guard,
            ));
            AppController::spawn_log_loop(log_rx, app.handle().clone());
            AppController::spawn_scan_result_loop(scan_result_rx, app.handle().clone());
            AppController::spawn_hotkey_loop(&controller);
            app.manage(controller);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
