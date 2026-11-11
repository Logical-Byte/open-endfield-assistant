//! 前台激活与窗口恢复。请求异步提交，流程按需要等待实际状态。

use super::wait_until;
use crate::Window;
use crate::windows::validate;
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, HWND_TOP, IsIconic, IsZoomed, SW_RESTORE, SWP_ASYNCWINDOWPOS, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindowAsync,
};
use anyhow::Context;

pub(crate) fn is_foreground(window: Window) -> bool {
    (unsafe { GetForegroundWindow() }) == window.raw
}

/// 请求激活并移到 Z 序顶部，不设为始终置顶，也不保证返回时已获得前台。
pub(crate) fn request_activation(window: Window) -> anyhow::Result<()> {
    validate(window)?;
    if !is_foreground(window) {
        unsafe {
            SetWindowPos(
                window.raw,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_ASYNCWINDOWPOS,
            )
        }?;
        unsafe { SetForegroundWindow(window.raw) }.ok()?;
    }
    Ok(())
}

pub(super) fn activate_and_wait(window: Window) -> anyhow::Result<()> {
    let request = request_activation(window);
    // 跨输入队列激活异步完成，以实际前台窗口为准。
    wait_until(|| Ok(is_foreground(window)))
        .with_context(|| format!("等待窗口获得前台焦点超时，激活请求结果：{request:?}"))
}

pub(super) fn is_minimized(window: Window) -> bool {
    unsafe { IsIconic(window.raw) }.as_bool()
}

pub(super) fn is_maximized(window: Window) -> bool {
    unsafe { IsZoomed(window.raw) }.as_bool()
}

/// 恢复最小化或最大化窗口，并等待恢复完成。
pub(super) fn restore(window: Window) -> anyhow::Result<()> {
    if !is_minimized(window) && !is_maximized(window) {
        return Ok(());
    }
    unsafe { ShowWindowAsync(window.raw, SW_RESTORE) }.ok()?;
    wait_until(|| Ok(!is_minimized(window) && !is_maximized(window)))
}

pub(crate) fn restore_if_minimized(window: Window) -> anyhow::Result<()> {
    validate(window)?;
    if is_minimized(window) {
        unsafe { ShowWindowAsync(window.raw, SW_RESTORE) }.ok()?;
        wait_until(|| Ok(!is_minimized(window)))?;
    }
    Ok(())
}
