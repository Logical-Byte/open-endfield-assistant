//! Windows 操作的所有权根。
//!
//! 公开 topic 模块定义 OEA 使用的稳定接口；原生 Windows 类型与重型实现
//! 集中在私有的 `details` 区域。

use windows::Win32::Foundation::HWND;

mod details;

pub mod admin;
pub mod capture;
pub mod dialog;
pub mod hotkey;
pub mod input;
pub mod webview2;
pub mod window;

/// OEA 持有的非拥有型窗口句柄。
///
/// 原生表示保持在 `windows_ops` 内；本类型不承诺跨线程安全。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WindowHandle(HWND);

impl WindowHandle {
    /// 句柄是否无效。
    pub fn is_invalid(self) -> bool {
        self.0.is_invalid()
    }
}
