//! 窗口状态读取、尺寸调整与等待原语，供窗口模式与内容区域调整共用。

use std::{
    thread,
    time::{Duration, Instant},
};

use ::windows::Win32::Foundation::{POINT, RECT};
use ::windows::Win32::Graphics::Gdi::{
    ClientToScreen, HMONITOR, MONITOR_DEFAULTTONEAREST, MonitorFromWindow,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, IsIconic, IsZoomed, SW_RESTORE, SWP_ASYNCWINDOWPOS,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, ShowWindowAsync,
};
use anyhow::{Context, ensure};

use crate::Window;

pub(crate) const POLL_INTERVAL: Duration = Duration::from_millis(50);
pub(crate) const PREPARE_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const SIZE_SETTLE_TIME: Duration = Duration::from_millis(300);

/// 客户区尺寸、屏幕位置，以及客户区相对窗口外框的边距。
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientFrame {
    pub(crate) width: i32,
    pub(crate) height: i32,
    /// 客户区左上角的屏幕坐标。
    pub(crate) origin: POINT,
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

/// 读取客户区尺寸，客户区为空时返回错误。
pub(crate) fn client_size(window: Window) -> anyhow::Result<(i32, i32)> {
    try_client_frame(window)?
        .map(|frame| (frame.width, frame.height))
        .context("窗口客户区尺寸无效")
}

/// 读取客户区尺寸与外框边距。
pub(crate) fn client_frame(window: Window) -> anyhow::Result<ClientFrame> {
    try_client_frame(window)?.context("窗口客户区尺寸无效")
}

/// 读取客户区尺寸与外框边距。窗口过渡期间客户区可能暂时为空，此时返回 `None`。
pub(crate) fn try_client_frame(window: Window) -> anyhow::Result<Option<ClientFrame>> {
    let hwnd = window.raw;
    let mut client = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client) }?;
    let width = client.right - client.left;
    let height = client.bottom - client.top;
    if width <= 0 || height <= 0 {
        return Ok(None);
    }

    let mut outer = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut outer) }?;
    let mut origin = POINT::default();
    unsafe { ClientToScreen(hwnd, &mut origin) }
        .ok()
        .context("转换客户区坐标失败")?;

    Ok(Some(ClientFrame {
        width,
        height,
        origin,
        left: origin.x - outer.left,
        top: origin.y - outer.top,
        right: outer.right - origin.x - width,
        bottom: outer.bottom - origin.y - height,
    }))
}

/// 按实测的外框与客户区差值，把客户区调整到指定物理像素尺寸。
///
/// 浏览器等自行处理 `WM_NCCALCSIZE` 的窗口，外框差值不能由
/// `AdjustWindowRectExForDpi` 预测，只能按当前实测差值反推。
pub(crate) fn set_client_size(window: Window, width: i32, height: i32) -> anyhow::Result<()> {
    let frame = client_frame(window)?;
    unsafe {
        SetWindowPos(
            window.raw,
            None,
            0,
            0,
            width + frame.left + frame.right,
            height + frame.top + frame.bottom,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
    }?;
    Ok(())
}

pub(crate) fn is_iconic(window: Window) -> bool {
    unsafe { IsIconic(window.raw) }.as_bool()
}

pub(crate) fn is_zoomed(window: Window) -> bool {
    unsafe { IsZoomed(window.raw) }.as_bool()
}

/// 恢复最小化或最大化的窗口，并等待恢复完成。
pub(crate) fn restore(window: Window) -> anyhow::Result<()> {
    if !is_iconic(window) && !is_zoomed(window) {
        return Ok(());
    }
    unsafe { ShowWindowAsync(window.raw, SW_RESTORE) }.ok()?;
    wait_until(|| Ok(!is_iconic(window) && !is_zoomed(window)))
}

/// 读取窗口所在显示器。
pub(crate) fn monitor(window: Window) -> anyhow::Result<HMONITOR> {
    let monitor = unsafe { MonitorFromWindow(window.raw, MONITOR_DEFAULTTONEAREST) };
    ensure!(!monitor.is_invalid(), "无法获取窗口所在显示器");
    Ok(monitor)
}

/// 反复求值直到返回 `true`，超时返回错误。
pub(crate) fn wait_until(mut ready: impl FnMut() -> anyhow::Result<bool>) -> anyhow::Result<()> {
    let started = Instant::now();
    loop {
        if ready()? {
            return Ok(());
        }
        ensure!(started.elapsed() < PREPARE_TIMEOUT, "等待窗口调整超时");
        thread::sleep(POLL_INTERVAL);
    }
}
