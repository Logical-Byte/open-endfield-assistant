//! 平台原语的所有权根。
//!
//! 公开 topic 模块定义 OEA 使用的稳定接口；原生 Windows 类型与重型实现
//! 集中在私有的 `windows` 区域。

#[cfg(target_os = "windows")]
use ::windows::Win32::Foundation::HWND;

#[cfg(target_os = "windows")]
mod windows;

pub mod admin;
pub mod capture;
pub mod data_protection;
pub mod dialog;
pub mod hotkey;
pub mod input;
#[cfg(target_os = "windows")]
pub mod registry;
pub mod sound;
pub mod webview;
pub mod window;

/// OEA 持有的非拥有型窗口句柄。
///
/// 原生表示保持在 `platform` 内；本类型不承诺跨线程安全。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WindowHandle {
    #[cfg(target_os = "windows")]
    raw: HWND,
}

impl WindowHandle {
    /// 句柄是否无效；macOS 开发外壳始终返回 `true`。
    pub fn is_invalid(self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.raw.is_invalid()
        }

        #[cfg(target_os = "macos")]
        {
            true
        }
    }
}

#[cfg(target_os = "macos")]
fn unsupported(operation: &str) -> anyhow::Error {
    anyhow::anyhow!("{operation} is not supported on macOS")
}
