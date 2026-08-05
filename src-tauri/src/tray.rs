//! 系统托盘：图标、菜单与"关闭时最小化到托盘"。
//!
//! 参考 [MXU 的 tray.rs](https://github.com/MistEO/MXU/blob/main/src-tauri/src/tray.rs) 实现：
//! - 托盘菜单直接驱动 [`Controller`]，无需前端中转（与热键分发同一模式）；
//! - 左键单击托盘图标显示主窗口；
//! - 窗口关闭请求由 [`handle_close_requested`] 决定是否隐藏到托盘。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};
use tracing::info;

use crate::controller::Controller;

/// 关闭窗口时是否最小化到托盘（默认开启，托盘菜单提供"退出"入口）。
static MINIMIZE_TO_TRAY: AtomicBool = AtomicBool::new(true);

/// 全局托盘图标引用，供后续动态更新图标 / tooltip。
static TRAY_ICON: OnceLock<Mutex<Option<TrayIcon>>> = OnceLock::new();

/// 设置"关闭窗口时最小化到托盘"。
pub fn set_minimize_to_tray(enabled: bool) {
    MINIMIZE_TO_TRAY.store(enabled, Ordering::SeqCst);
    info!("最小化到托盘: {enabled}");
}

/// 查询"关闭窗口时最小化到托盘"。
pub fn get_minimize_to_tray() -> bool {
    MINIMIZE_TO_TRAY.load(Ordering::SeqCst)
}

/// 显示并聚焦主窗口（最小化时先还原）。
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 处理窗口关闭请求：启用最小化到托盘时隐藏窗口并阻止关闭。
///
/// 返回 `true` 表示已阻止关闭（应用继续驻留托盘），`false` 表示允许关闭。
pub fn handle_close_requested(app: &AppHandle) -> bool {
    if get_minimize_to_tray() {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        true
    } else {
        false
    }
}

/// 初始化系统托盘（在 `setup` 中、`Controller` 托管之后调用）。
pub fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 托盘菜单项
    let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let start_i = MenuItem::with_id(app, "start", "开始任务", true, None::<&str>)?;
    let stop_i = MenuItem::with_id(app, "stop", "停止任务", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &start_i, &stop_i, &quit_i])?;

    // 图标：复用应用图标（tauri.conf.json 的 bundle.icon）
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("未找到应用图标，无法创建托盘图标")?;

    let tray = TrayIconBuilder::<Wry>::new()
        .icon(icon)
        .tooltip("OEA")
        .menu(&menu)
        // 左键单击不弹菜单，由 on_tray_icon_event 处理为"显示主窗口"
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "start" => {
                if let Some(controller) = app.try_state::<Arc<Controller>>() {
                    controller.start_scan();
                }
            }
            "stop" => {
                if let Some(controller) = app.try_state::<Arc<Controller>>() {
                    controller.stop_scan();
                }
            }
            "quit" => {
                if let Some(controller) = app.try_state::<Arc<Controller>>() {
                    controller.quit();
                } else {
                    app.exit(0);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击显示主窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    // 保存托盘引用，供后续动态更新
    let tray_mutex = TRAY_ICON.get_or_init(|| Mutex::new(None));
    let mut guard = tray_mutex.lock().map_err(|_| "托盘图标互斥锁中毒")?;
    *guard = Some(tray);

    info!("系统托盘初始化完成");
    Ok(())
}
