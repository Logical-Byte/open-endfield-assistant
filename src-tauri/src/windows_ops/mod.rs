//! Windows 操作的所有权根。
//!
//! 公开 topic 模块只负责提供稳定的 OEA 模块路径；具体实现集中在私有的
//! `details` 区域。现阶段 topic 接口仍保留已有的 raw Windows 类型，后续
//! stack 层再逐步替换这些接口。

use windows::Win32::Foundation::HWND;

mod details;

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

pub mod admin {
    pub use super::details::admin::*;
}

pub mod capture {
    pub use super::details::capture::*;
}

pub mod dialog {
    pub use super::details::dialog::*;
}

pub mod hotkey {
    pub use super::details::hotkey::*;
}

pub mod input {
    pub use super::details::input::*;
}

pub mod webview2 {
    pub use super::details::webview2::*;
}

pub mod window {
    pub use super::details::window::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_native_handle_is_invalid() {
        assert!(WindowHandle(HWND::default()).is_invalid());
    }
}
