//! Windows 窗口接口。

use anyhow::Result;

use crate::utils::region::Region2D;

use super::{WindowHandle, details};

/// 终末地游戏窗口标题。
pub const ENDFIELD_WINDOW_TITLE: &str = "Endfield";
/// 终末地游戏窗口类名。
pub const ENDFIELD_WINDOW_CLASS: &str = "UnityWndClass";

/// 判断全局热键触发时前台窗口是否为 OEA 或终末地。
pub struct ForegroundGuard {
    oea_window: WindowHandle,
}

// 窗口句柄由热键消费线程串行访问。
unsafe impl Send for ForegroundGuard {}
unsafe impl Sync for ForegroundGuard {}

impl ForegroundGuard {
    /// 绑定 OEA 自身的窗口句柄。
    pub fn new(oea_window: WindowHandle) -> Self {
        Self { oea_window }
    }

    /// 当前前台窗口是否为 OEA 自身或终末地。
    pub fn is_foreground_eligible(&self) -> bool {
        let foreground = get_foreground_window();
        if foreground == self.oea_window {
            return true;
        }

        match get_window_by_title(Some(ENDFIELD_WINDOW_CLASS), Some(ENDFIELD_WINDOW_TITLE)) {
            Ok(game_window) if !game_window.is_invalid() => foreground == game_window,
            _ => false,
        }
    }
}

/// 获取 OEA 主窗口的原生窗口句柄。
pub fn get_app_window(app_handle: &tauri::AppHandle) -> Result<WindowHandle> {
    details::window::get_app_window(app_handle)
}

fn get_foreground_window() -> WindowHandle {
    details::window::get_foreground_window()
}

/// 按窗口类名和标题查找窗口。
pub fn get_window_by_title(class_name: Option<&str>, title: Option<&str>) -> Result<WindowHandle> {
    details::window::get_window_by_title(class_name, title)
}

/// 获取窗口客户区矩形。
pub fn get_client_rect(window: WindowHandle) -> Result<Region2D<i32>> {
    details::window::get_client_rect(window)
}

/// 将当前线程设置为 Per Monitor v2 DPI 感知。
pub fn set_thread_dpi_awareness_context() {
    details::window::set_thread_dpi_awareness_context();
}

/// 激活窗口并将其置于最前。
pub fn ensure_foreground_and_topmost(window: WindowHandle) -> Result<()> {
    details::window::ensure_foreground_and_topmost(window)
}

/// 窗口最小化时将其恢复。
pub fn restore_window_if_minimized(window: WindowHandle) -> Result<()> {
    details::window::restore_window_if_minimized(window)
}

/// 确保窗口客户区位于显示器可见范围内。
pub fn ensure_window_on_screen(window: WindowHandle) -> Result<()> {
    details::window::ensure_window_on_screen(window)
}

pub mod hdr {
    use anyhow::Result;

    use super::{WindowHandle, details};

    /// 判断窗口所在显示器是否开启 HDR。
    pub fn is_hdr_enabled_on_window_monitor(window: WindowHandle) -> Result<bool> {
        details::window::hdr::is_hdr_enabled_on_window_monitor(window)
    }
}
