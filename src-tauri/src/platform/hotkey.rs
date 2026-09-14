//! 全局键盘监听接口。

use std::sync::mpsc;

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::windows;

/// Delete 键的 OEA 键码。
pub const DELETE_KEY: u32 = 0x2e;
/// `'` 键的 OEA 键码。
pub const OEM_7_KEY: u32 = 0xde;
/// Alt 修饰键的 OEA 状态位。
pub const ALT_MODIFIER: u32 = 1 << 0;

/// 一次原始键盘事件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEvent {
    /// Windows 虚拟键码。
    pub vk: u32,
    /// `true` 表示按下，`false` 表示弹起。
    pub down: bool,
    /// Alt、Ctrl、Shift 和 Win 的状态位快照。
    pub modifiers: u32,
}

/// 全局键盘监听线程的释放守卫。
pub struct KeyboardHookGuard {
    #[cfg(target_os = "windows")]
    inner: windows::hotkey::KeyboardHookGuard,
}

impl KeyboardHookGuard {
    /// 停止键盘消息泵并等待监听线程退出。
    pub fn shutdown(self) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            self.inner.shutdown()
        }

        #[cfg(target_os = "macos")]
        {
            Ok(())
        }
    }
}

/// 启动只感知、不拦截按键的全局键盘监听。
///
/// 自动重复在监听层过滤，返回端只收到首次按下与弹起事件。
/// macOS 开发外壳返回一个不会产生事件的接收端。
pub fn listen() -> Result<(mpsc::Receiver<KeyEvent>, KeyboardHookGuard)> {
    #[cfg(target_os = "windows")]
    {
        let (rx, inner) = windows::hotkey::listen()?;
        Ok((rx, KeyboardHookGuard { inner }))
    }

    #[cfg(target_os = "macos")]
    {
        let (_tx, rx) = mpsc::channel();
        Ok((rx, KeyboardHookGuard {}))
    }
}
