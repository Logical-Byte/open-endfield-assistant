use ::windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use anyhow::Context;
use oea_window::Window;
use tauri::Manager;

pub(in crate::platform) fn app_window(app_handle: &tauri::AppHandle) -> anyhow::Result<Window> {
    let window = app_handle
        .get_webview_window("main")
        .context("未找到 OEA 主窗口")?;
    Window::from_hwnd(window.hwnd()?)
}

pub(in crate::platform) fn set_thread_dpi_awareness_context() {
    let _ = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}
