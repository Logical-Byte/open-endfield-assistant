//! Windows 窗口接口。

use anyhow::Result;

use crate::utils::region::Region2D;

use super::WindowHandle;

#[cfg(target_os = "windows")]
use super::details;

/// 终末地游戏窗口标题。
pub const ENDFIELD_WINDOW_TITLE: &str = "Endfield";
/// 终末地游戏窗口类名。
pub const ENDFIELD_WINDOW_CLASS: &str = "UnityWndClass";

/// 判断全局热键触发时前台窗口是否为 OEA 或终末地。
#[cfg(target_os = "windows")]
pub struct ForegroundGuard {
    oea_window: WindowHandle,
}

/// macOS 开发外壳使用的空前台窗口守卫。
#[cfg(target_os = "macos")]
pub struct ForegroundGuard;

// 窗口句柄由热键消费线程串行访问。
unsafe impl Send for ForegroundGuard {}
unsafe impl Sync for ForegroundGuard {}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "macos")]
impl ForegroundGuard {
    /// 构造不持有原生资源的 macOS 前台窗口守卫。
    pub fn new(_oea_window: WindowHandle) -> Self {
        Self
    }

    /// macOS 开发外壳不响应 Windows 全局热键。
    pub fn is_foreground_eligible(&self) -> bool {
        false
    }
}

/// 获取 OEA 主窗口的原生窗口句柄。
#[cfg(target_os = "windows")]
pub fn get_app_window(app_handle: &tauri::AppHandle) -> Result<WindowHandle> {
    details::window::get_app_window(app_handle)
}

/// macOS 开发外壳不需要主窗口的 Windows 句柄。
#[cfg(target_os = "macos")]
pub fn get_app_window(_app_handle: &tauri::AppHandle) -> Result<WindowHandle> {
    Ok(WindowHandle)
}

#[cfg(target_os = "windows")]
fn get_foreground_window() -> WindowHandle {
    details::window::get_foreground_window()
}

/// 按窗口类名和标题查找窗口。
#[cfg(target_os = "windows")]
pub fn get_window_by_title(class_name: Option<&str>, title: Option<&str>) -> Result<WindowHandle> {
    details::window::get_window_by_title(class_name, title)
}

/// macOS 开发外壳不支持查找 Windows 游戏窗口。
#[cfg(target_os = "macos")]
pub fn get_window_by_title(
    _class_name: Option<&str>,
    _title: Option<&str>,
) -> Result<WindowHandle> {
    Err(super::unsupported("window lookup"))
}

/// 获取窗口客户区矩形。
#[cfg(target_os = "windows")]
pub fn get_client_rect(window: WindowHandle) -> Result<Region2D<i32>> {
    details::window::get_client_rect(window)
}

/// macOS 开发外壳不提供 Windows 客户区坐标。
#[cfg(target_os = "macos")]
pub fn get_client_rect(_window: WindowHandle) -> Result<Region2D<i32>> {
    Err(super::unsupported("window geometry"))
}

/// 将当前线程设置为 Per Monitor v2 DPI 感知。
#[cfg(target_os = "windows")]
pub fn set_thread_dpi_awareness_context() {
    details::window::set_thread_dpi_awareness_context();
}

/// macOS 开发外壳无需设置 Windows DPI 感知。
#[cfg(target_os = "macos")]
pub fn set_thread_dpi_awareness_context() {}

/// 激活窗口并将其置于最前。
#[cfg(target_os = "windows")]
pub fn ensure_foreground_and_topmost(window: WindowHandle) -> Result<()> {
    details::window::ensure_foreground_and_topmost(window)
}

/// macOS 开发外壳不支持 Windows 前台窗口控制。
#[cfg(target_os = "macos")]
pub fn ensure_foreground_and_topmost(_window: WindowHandle) -> Result<()> {
    Err(super::unsupported("foreground window control"))
}

/// 窗口最小化时将其恢复。
#[cfg(target_os = "windows")]
pub fn restore_window_if_minimized(window: WindowHandle) -> Result<()> {
    details::window::restore_window_if_minimized(window)
}

/// macOS 开发外壳不支持恢复 Windows 游戏窗口。
#[cfg(target_os = "macos")]
pub fn restore_window_if_minimized(_window: WindowHandle) -> Result<()> {
    Err(super::unsupported("window restoration"))
}

/// 确保窗口客户区位于显示器可见范围内。
#[cfg(target_os = "windows")]
pub fn ensure_window_on_screen(window: WindowHandle) -> Result<()> {
    details::window::ensure_window_on_screen(window)
}

/// macOS 开发外壳不支持调整 Windows 游戏窗口位置。
#[cfg(target_os = "macos")]
pub fn ensure_window_on_screen(_window: WindowHandle) -> Result<()> {
    Err(super::unsupported("window positioning"))
}

pub mod hdr {
    use anyhow::Result;

    use super::WindowHandle;

    #[cfg(target_os = "windows")]
    use super::details;

    /// 判断窗口所在显示器是否开启 HDR。
    #[cfg(target_os = "windows")]
    pub fn is_hdr_enabled_on_window_monitor(window: WindowHandle) -> Result<bool> {
        details::window::hdr::is_hdr_enabled_on_window_monitor(window)
    }

    /// macOS 开发外壳不检测 Windows 游戏窗口的 HDR 状态。
    #[cfg(target_os = "macos")]
    pub fn is_hdr_enabled_on_window_monitor(_window: WindowHandle) -> Result<bool> {
        Err(super::super::unsupported("HDR detection"))
    }
}
