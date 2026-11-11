//! 窗口客户区、外框与显示器工作区的几何读取和调整。
//! `windows::geometry` 负责原生类型转换，本 module 负责窗口几何操作。

use super::{SIZE_SETTLE_TIME, activation, wait_until};
use crate::Window;
use crate::windows::validate;
use ::windows::Win32::Foundation::{POINT, RECT};
use ::windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromWindow,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowPos,
};
use anyhow::{Context, ensure};
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub(super) struct ClientGeometry {
    pub(super) client_size: (i32, i32),
    /// 客户区左上角的屏幕坐标。
    pub(super) client_origin: POINT,
    pub(super) outer_rect: RECT,
}

impl ClientGeometry {
    pub(super) fn outer_size(&self) -> (i32, i32) {
        (
            self.outer_rect.right - self.outer_rect.left,
            self.outer_rect.bottom - self.outer_rect.top,
        )
    }

    fn outer_size_for_client(&self, width: i32, height: i32) -> (i32, i32) {
        let (outer_width, outer_height) = self.outer_size();
        (
            outer_width - self.client_size.0 + width,
            outer_height - self.client_size.1 + height,
        )
    }
}

pub(crate) fn client_size(window: Window) -> anyhow::Result<(i32, i32)> {
    Ok(client_geometry(window)?.client_size)
}

fn client_geometry(window: Window) -> anyhow::Result<ClientGeometry> {
    try_client_geometry(window)?.context("窗口客户区尺寸无效")
}

/// 窗口过渡期间客户区可能暂时为空，此时返回 `None`。
pub(super) fn try_client_geometry(window: Window) -> anyhow::Result<Option<ClientGeometry>> {
    let mut client = RECT::default();
    unsafe { GetClientRect(window.raw, &mut client) }?;
    let client_size = (client.right - client.left, client.bottom - client.top);
    if client_size.0 <= 0 || client_size.1 <= 0 {
        return Ok(None);
    }
    let mut outer_rect = RECT::default();
    unsafe { GetWindowRect(window.raw, &mut outer_rect) }?;
    let mut client_origin = POINT::default();
    unsafe { ClientToScreen(window.raw, &mut client_origin) }
        .ok()
        .context("转换客户区坐标失败")?;
    Ok(Some(ClientGeometry {
        client_size,
        client_origin,
        outer_rect,
    }))
}

/// 按实测外框差值发出异步调整请求，适用于自绘非客户区的浏览器窗口。
pub(super) fn set_client_size(window: Window, width: i32, height: i32) -> anyhow::Result<()> {
    let (outer_width, outer_height) = client_geometry(window)?.outer_size_for_client(width, height);
    unsafe {
        SetWindowPos(
            window.raw,
            None,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
    }?;
    Ok(())
}

/// 调整客户区并等待连续稳定，调用方负责先完成目标专属的窗口化过渡。
pub(crate) fn resize_client(window: Window, width: i32, height: i32) -> anyhow::Result<()> {
    ensure!(width > 0 && height > 0, "窗口客户区尺寸必须为正数");
    validate(window)?;
    if client_size(window)? != (width, height) {
        set_client_size(window, width, height)?;
    }
    // 窗口化流程已确认外框恢复，尺寸请求只发一次，随后等待消息处理与几何稳定。
    let mut stable_since = None;
    wait_until(|| {
        let current = try_client_geometry(window)?.map(|frame| frame.client_size);
        if current == Some((width, height)) {
            Ok(stable_since.get_or_insert_with(Instant::now).elapsed() >= SIZE_SETTLE_TIME)
        } else {
            stable_since = None;
            Ok(false)
        }
    })
    .context("窗口未能稳定到指定的客户区尺寸")
}

pub(super) fn monitor(window: Window) -> anyhow::Result<HMONITOR> {
    let monitor = unsafe { MonitorFromWindow(window.raw, MONITOR_DEFAULTTONEAREST) };
    ensure!(!monitor.is_invalid(), "无法获取窗口所在显示器");
    Ok(monitor)
}

/// 调整客户区相对工作区的位置，保留尺寸时允许客户区超出工作区，不检查其他窗口遮挡。
pub(crate) fn adjust_client_to_work_area(
    window: Window,
    preserve_size: bool,
) -> anyhow::Result<()> {
    validate(window)?;
    if activation::is_maximized(window) || activation::is_minimized(window) {
        return Ok(());
    }
    let monitor = monitor(window)?;
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut info) }.ok()?;
    let work = info.rcWork;
    let (width, height) = client_size(window)?;
    if !preserve_size && (width > work.right - work.left || height > work.bottom - work.top) {
        resize_client(
            window,
            width.min(work.right - work.left),
            height.min(work.bottom - work.top),
        )?;
    }
    let frame = client_geometry(window)?;
    let (width, height) = frame.client_size;
    let x = frame.client_origin.x.min(work.right - width).max(work.left);
    let y = frame
        .client_origin
        .y
        .min(work.bottom - height)
        .max(work.top);
    if x == frame.client_origin.x && y == frame.client_origin.y {
        return Ok(());
    }
    let outer_x = x - (frame.client_origin.x - frame.outer_rect.left);
    let outer_y = y - (frame.client_origin.y - frame.outer_rect.top);
    unsafe {
        SetWindowPos(
            window.raw,
            None,
            outer_x,
            outer_y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
    }?;
    Ok(())
}
