use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use ::windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use anyhow::Context;
use tauri::Manager;

/// 仅用于 OEA 主窗口的非拥有型句柄，原生类型不离开平台 implementation。
pub(in crate::platform) struct AppWindow(HWND);

// 非拥有型 HWND 只用于热键线程的前台比较，不操作 Tauri 对象。
unsafe impl Send for AppWindow {}

impl AppWindow {
    pub(in crate::platform) fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        let window = app_handle
            .get_webview_window("main")
            .context("未找到 OEA 主窗口")?;
        Ok(Self(window.hwnd()?))
    }

    pub(in crate::platform) fn is_foreground(&self) -> bool {
        (unsafe { GetForegroundWindow() }) == self.0
    }
}

pub(in crate::platform) fn set_thread_dpi_awareness_context() {
    let _ = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}
