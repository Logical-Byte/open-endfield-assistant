//! 窗口平台接口。

use anyhow::Result;

use crate::utils::region::Region2D;

use super::WindowHandle;

#[cfg(target_os = "windows")]
use super::windows;

/// 终末地游戏窗口标题。
pub const ENDFIELD_WINDOW_TITLE: &str = "Endfield";
/// 终末地游戏窗口类名。
pub const ENDFIELD_WINDOW_CLASS: &str = "UnityWndClass";

/// 判断全局热键触发时前台窗口是否为 OEA 或终末地。
pub struct ForegroundGuard {
    #[cfg(target_os = "windows")]
    oea_window: WindowHandle,
}

// 窗口句柄由热键消费线程串行访问。
unsafe impl Send for ForegroundGuard {}
unsafe impl Sync for ForegroundGuard {}

impl ForegroundGuard {
    /// 绑定 OEA 自身的窗口句柄；macOS 开发外壳不持有原生资源。
    pub fn new(oea_window: WindowHandle) -> Self {
        #[cfg(target_os = "windows")]
        {
            Self { oea_window }
        }

        #[cfg(target_os = "macos")]
        {
            let _ = oea_window;
            Self {}
        }
    }

    /// 当前前台窗口是否为 OEA 自身或终末地；macOS 开发外壳返回 `false`。
    pub fn is_foreground_eligible(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            let foreground = get_foreground_window();
            if foreground == self.oea_window {
                return true;
            }

            match get_window_by_title(Some(ENDFIELD_WINDOW_CLASS), Some(ENDFIELD_WINDOW_TITLE)) {
                Ok(game_window) if !game_window.is_invalid() => foreground == game_window,
                _ => false,
            }
        }

        #[cfg(target_os = "macos")]
        {
            false
        }
    }
}

/// 获取 OEA 主窗口的原生窗口句柄；macOS 开发外壳返回空句柄。
pub fn get_app_window(app_handle: &tauri::AppHandle) -> Result<WindowHandle> {
    #[cfg(target_os = "windows")]
    {
        windows::window::get_app_window(app_handle)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app_handle;
        Ok(WindowHandle {})
    }
}

#[cfg(target_os = "windows")]
fn get_foreground_window() -> WindowHandle {
    windows::window::get_foreground_window()
}

/// 按窗口类名和标题查找窗口；macOS 开发外壳返回 unsupported error。
pub fn get_window_by_title(class_name: Option<&str>, title: Option<&str>) -> Result<WindowHandle> {
    #[cfg(target_os = "windows")]
    {
        windows::window::get_window_by_title(class_name, title)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (class_name, title);
        Err(super::unsupported("window lookup"))
    }
}

/// 获取窗口客户区矩形；macOS 开发外壳返回 unsupported error。
pub fn get_client_rect(window: WindowHandle) -> Result<Region2D<i32>> {
    #[cfg(target_os = "windows")]
    {
        windows::window::get_client_rect(window)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window;
        Err(super::unsupported("window geometry"))
    }
}

/// 将当前线程设置为 Per Monitor v2 DPI 感知；macOS 开发外壳不执行任何操作。
pub fn set_thread_dpi_awareness_context() {
    #[cfg(target_os = "windows")]
    {
        windows::window::set_thread_dpi_awareness_context();
    }

    #[cfg(target_os = "macos")]
    {}
}

/// 激活窗口并将其置于最前；macOS 开发外壳返回 unsupported error。
pub fn ensure_foreground_and_topmost(window: WindowHandle) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::window::ensure_foreground_and_topmost(window)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window;
        Err(super::unsupported("foreground window control"))
    }
}

/// 窗口最小化时将其恢复；macOS 开发外壳返回 unsupported error。
pub fn restore_window_if_minimized(window: WindowHandle) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::window::restore_window_if_minimized(window)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window;
        Err(super::unsupported("window restoration"))
    }
}

/// 确保窗口客户区位于显示器可见范围内；macOS 开发外壳返回 unsupported error。
pub fn ensure_window_on_screen(window: WindowHandle) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::window::ensure_window_on_screen(window)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window;
        Err(super::unsupported("window positioning"))
    }
}

pub mod hdr {
    use anyhow::Result;

    use super::WindowHandle;

    #[cfg(target_os = "windows")]
    use super::windows;

    /// 判断窗口所在显示器是否开启 HDR；macOS 开发外壳返回 unsupported error。
    pub fn is_hdr_enabled_on_window_monitor(window: WindowHandle) -> Result<bool> {
        #[cfg(target_os = "windows")]
        {
            windows::window::hdr::is_hdr_enabled_on_window_monitor(window)
        }

        #[cfg(target_os = "macos")]
        {
            let _ = window;
            Err(super::super::unsupported("HDR detection"))
        }
    }
}
