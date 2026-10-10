//! 系统托盘：图标、菜单与"关闭时最小化到托盘"。
//!
//! - 托盘菜单直接驱动 [`Controller`]，无需前端中转（与热键分发同一模式）；
//! - 左键单击托盘图标显示主窗口；

use std::sync::{Mutex, OnceLock};

use anyhow::Context;
use tauri::{
    AppHandle, Listener, Manager, Wry,
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};
use tracing::info;

use crate::{controller::Controller, locale::UiLocale};

// 托盘只消费运行阶段，无需从 IPC 还原内部错误及其诊断来源。
#[derive(serde::Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum AutomationState {
    Idle,
    Running,
    Stopping,
}

/// 全局托盘图标引用，供后续动态更新图标 / tooltip。
static TRAY_ICON: OnceLock<Mutex<Option<TrayIcon>>> = OnceLock::new();

/// 菜单句柄统一保存，刷新只改变文字，不替换菜单操作。
struct TrayItems {
    show: MenuItem<Wry>,
    toggle: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}
static TRAY_ITEMS: OnceLock<Mutex<Option<TrayItems>>> = OnceLock::new();

fn menu_text(locale: UiLocale, running: bool) -> (&'static str, &'static str, &'static str) {
    match locale {
        UiLocale::ZhCn => (
            "显示主窗口",
            if running {
                "停止扫描"
            } else {
                "开始扫描"
            },
            "退出",
        ),
        UiLocale::EnUs => (
            "Show main window",
            if running { "Stop scan" } else { "Start scan" },
            "Quit",
        ),
    }
}

/// 显示并聚焦主窗口（最小化时先还原）。
fn show_main_window(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 按有效应用语言刷新菜单，并让扫描菜单文案与任务运行状态同步。
fn update_items(app: &AppHandle, running: bool) {
    let Some(controller) = app.try_state::<Controller>() else {
        return;
    };
    let text = menu_text(controller.settings_snapshot().ui_locale, running);
    let Some(items) = TRAY_ITEMS.get() else {
        return;
    };
    let Ok(guard) = items.lock() else {
        return;
    };
    if let Some(items) = guard.as_ref() {
        let _ = items.show.set_text(text.0);
        let _ = items.toggle.set_text(text.1);
        let _ = items.quit.set_text(text.2);
    }
}

/// 设置成功提交后刷新托盘语言，自动化状态和菜单动作保持原样。
pub(super) fn refresh_locale(app: &AppHandle) {
    if let Some(controller) = app.try_state::<Controller>() {
        update_items(app, controller.automation_status().is_active());
    }
}

/// 初始化系统托盘（在 `setup` 中、`Controller` 托管之后调用）。
pub fn init_tray(app_handle: &AppHandle) -> anyhow::Result<()> {
    let controller = app_handle.state::<Controller>();
    let text = menu_text(
        controller.settings_snapshot().ui_locale,
        controller.automation_status().is_active(),
    );
    // 托盘菜单项：开始/停止扫描合并为一个动态切换项
    let show_item = MenuItem::with_id(app_handle, "show", text.0, true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(app_handle, "toggle", text.1, true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app_handle, "quit", text.2, true, None::<&str>)?;

    let menu = MenuBuilder::new(app_handle)
        .item(&show_item)
        .item(&toggle_item)
        .separator()
        .item(&quit_item)
        .build()?;

    // 图标：复用应用图标（tauri.conf.json 的 bundle.icon）
    let icon = app_handle
        .default_window_icon()
        .cloned()
        .context("未找到应用图标，无法创建托盘图标")?;

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("OEA")
        .menu(&menu)
        // 左键单击不弹菜单，由 on_tray_icon_event 处理为"显示主窗口"
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "toggle" => {
                if let Some(controller) = app.try_state::<Controller>() {
                    controller.toggle_archive_scan();
                }
            }
            "quit" => {
                if let Some(controller) = app.try_state::<Controller>() {
                    controller.quit(app);
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
        .build(app_handle)?;

    // 保存托盘引用，供后续动态更新
    {
        let tray_mutex = TRAY_ICON.get_or_init(|| Mutex::new(None));
        let mut guard = tray_mutex.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(tray);
    }

    {
        let items = TRAY_ITEMS.get_or_init(|| Mutex::new(None));
        *items.lock().unwrap_or_else(|e| e.into_inner()) = Some(TrayItems {
            show: show_item,
            toggle: toggle_item,
            quit: quit_item,
        });
    }

    // 订阅运行状态事件：扫描档案库任务启动 / 结束都会推送，据此切换菜单文案
    let event_app = app_handle.clone();
    app_handle.listen("automation-status-changed", move |event| {
        if let Ok(state) = serde_json::from_str::<AutomationState>(event.payload()) {
            update_items(&event_app, !matches!(state, AutomationState::Idle));
        }
    });
    // 同步初始状态。
    if let Some(controller) = app_handle.try_state::<Controller>() {
        update_items(app_handle, controller.automation_status().is_active());
    }

    info!("系统托盘初始化完成");
    Ok(())
}
