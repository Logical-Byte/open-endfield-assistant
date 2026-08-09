//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

pub mod app_paths;
pub mod config;
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
pub mod tray;
pub mod types;
pub mod utils;
pub mod window;

use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex, mpsc};

use anyhow::{Context, Result, anyhow};
use rapidocr_core::config::PipelineConfig;
use tauri::Manager;
use tracing::info;
use windows::Win32::Foundation::HWND;

use crate::{
    app_paths::AppPaths,
    controller::Controller,
    ocr::OcrEngine,
    scene::create_scene_manager,
    tasks::archive_scan::CorrectionIndex,
    types::{ArchiveAcquisitionContract, PrtsData},
    window::ForegroundGuard,
};

/// 获取 OEA 主窗口的原生窗口句柄（用于前台窗口判定）。
fn get_oea_hwnd(app_handle: &tauri::AppHandle) -> Result<HWND> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| anyhow!("未找到 OEA 主窗口"))?;
    Ok(window.hwnd()?)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // WebView2 默认通过 raw input 接收键盘输入，当 OEA 窗口聚焦时会导致
        // WH_KEYBOARD_LL 低级键盘钩子收不到按键（[`tauri-apps/tauri#13919`](https://github.com/tauri-apps/tauri/issues/13919)）。
        // Always = 移除 raw input 注册，让 LL 钩子全局都能收到按键。
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            tauri_commands::start_scan,
            tauri_commands::stop_scan,
            tauri_commands::get_status,
            tauri_commands::get_prts_data,
            tauri_commands::get_archive_acquisition_contract,
            tauri_commands::quit,
            tauri_commands::open_log_dir,
            tauri_commands::load_oea_config,
            tauri_commands::save_oea_config,
            tauri_commands::screenshot,
        ])
        .on_window_event(|window, event| {
            // 关闭窗口时：若启用最小化到托盘，则隐藏窗口而不是退出应用
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
            // 解析资源目录（resources/models/logs），不依赖运行时工作目录
            let app_paths = AppPaths::new()?;

            // 初始化日志系统：控制台输出 DEBUG+，文件输出 TRACE+，前端转发 TRACE+（界面可过滤等级）。
            let (logger_guard, log_rx) = logger::init(&app_paths.logs_dir());

            // 解析应用配置文件
            let oea_config = config::load_oea_config(&app_paths.oea_config_file());

            // 绿色便携：WebView2 用户数据目录放在应用目录内（默认会写入 `%LOCALAPPDATA%\<identifier>`），保证所有磁盘写入都限定在应用目录内。
            fs::create_dir_all(app_paths.webview_data_dir())?;
            // 在 Rust 里动态创建 webview 窗口，而不在 `tauri.conf.json` 里声明窗口，否则无法更改 WebView2 用户数据目录。
            let main_window_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                    .title("OEA")
                    .inner_size(1024.0, 640.0)
                    .min_inner_size(256.0, 192.0)
                    .resizable(true)
                    .decorations(true)
                    .shadow(true)
                    .data_directory(app_paths.webview_data_dir());

            // 方案2：Windows 下移除系统标题栏，改用前端自定义标题栏（随应用主题融入）；
            // macOS/Linux 保留原生标题栏。
            #[cfg(target_os = "windows")]
            let main_window_builder = main_window_builder.decorations(false);

            let _main_window = main_window_builder.build()?;

            // 设置线程 DPI 感知上下文，确保截图器获取的窗口客户区坐标与实际像素一致。
            window::set_thread_dpi_awareness_context();

            // 扫描结果通道：任务线程产生 → 转发线程 emit 给前端
            let (scan_tx, scan_rx) = mpsc::channel();
            let scan_index = Arc::new(AtomicU32::new(0));

            // 初始化 OCR 引擎（不依赖游戏窗口，任务开始时复用）
            let pipeline_config = PipelineConfig::recognition_only();
            let ocr_engine = OcrEngine::new(pipeline_config, &app_paths.models_dir())?;
            let ocr = Arc::new(Mutex::new(ocr_engine));

            // 加载 prts.json：一次读取，同时用于纠错索引（后端）与前端数据查询
            let prts_path = app_paths.resources_dir().join("data/prts.json");
            let prts_text = fs::read_to_string(&prts_path).context("读取 prts.json 失败")?;
            let prts = Arc::new(
                serde_json::from_str::<PrtsData>(&prts_text).context("解析 prts.json 失败")?,
            );
            let correction = Arc::new(CorrectionIndex::from_prts(&prts));
            info!("已加载 prts.json（{} 个档案条目）", correction.len());

            // 加载档案获取契约：供前端按档案 id 展示获取方式
            let contract_path = app_paths
                .resources_dir()
                .join("data/archive_acquisition_contract.json");
            let contract_text = fs::read_to_string(&contract_path)
                .context("读取 archive_acquisition_contract.json 失败")?;
            let archive_acquisition_contract = Arc::new(
                serde_json::from_str::<ArchiveAcquisitionContract>(&contract_text)
                    .context("解析 archive_acquisition_contract.json 失败")?,
            );
            info!(
                "已加载 archive_acquisition_contract.json（{} 条获取契约）",
                archive_acquisition_contract.len()
            );

            // 场景管理器（本游戏全部场景，注册顺序即识别优先级）
            let scenes = Arc::new(create_scene_manager());

            // 开始监听热键
            let oea_hwnd = get_oea_hwnd(app.handle())?;
            let foreground = ForegroundGuard::new(oea_hwnd);
            let hotkey_rx = hotkey::listen()?;

            // 状态标志（Controller 唯一归属）
            let stop = Arc::new(AtomicBool::new(false));
            let running = Arc::new(AtomicBool::new(false));

            // 组装 Controller 并托管为 State，启动后台线程
            let controller = Arc::new(Controller::new(
                app_paths,
                Mutex::new(oea_config),
                ocr,
                scenes,
                stop,
                running,
                scan_tx,
                scan_index,
                foreground,
                app.handle().clone(),
                prts,
                archive_acquisition_contract,
                correction,
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
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            let (_, _) = (app_handle, event);
            // dbg!(app_handle, event);
        });
}
