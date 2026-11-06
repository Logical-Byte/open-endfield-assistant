//! OEA 主窗口与 Tauri 的平台接入。

#[cfg(windows)]
use super::windows;
use oea_window::Window;

pub fn app_window(app_handle: &tauri::AppHandle) -> anyhow::Result<Window> {
    #[cfg(windows)]
    {
        windows::window::app_window(app_handle)
    }
    #[cfg(not(windows))]
    {
        let _ = app_handle;
        Err(super::unsupported("app window access"))
    }
}

/// 设置 OEA 应用线程的 DPI 上下文。游戏窗口操作由 `oea-window` 自行管理。
pub fn set_thread_dpi_awareness_context() {
    #[cfg(windows)]
    windows::window::set_thread_dpi_awareness_context();
}
