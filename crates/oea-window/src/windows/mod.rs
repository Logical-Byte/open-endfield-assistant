pub(crate) mod capture;
mod geometry;
pub(crate) mod input;
pub(crate) mod window;

use ::windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use ::windows::Win32::UI::WindowsAndMessaging::IsWindow;
use anyhow::ensure;

use crate::Window;

pub(crate) fn validate(window: Window) -> anyhow::Result<()> {
    ensure!(
        unsafe { IsWindow(Some(window.raw)) }.as_bool(),
        "目标窗口已失效"
    );
    Ok(())
}

/// 只在当前同步调用中改变 DPI 上下文，退出时恢复，不要求调用方设置线程状态。
pub(crate) struct DpiContext(DPI_AWARENESS_CONTEXT);

impl DpiContext {
    pub(crate) fn enter() -> anyhow::Result<Self> {
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        ensure!(!previous.0.is_null(), "设置线程 DPI 上下文失败");
        Ok(Self(previous))
    }
}

impl Drop for DpiContext {
    fn drop(&mut self) {
        let _ = unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}
