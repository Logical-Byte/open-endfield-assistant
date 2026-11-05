use anyhow::bail;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetForegroundWindow, GetWindowRect, HWND_TOP, IsIconic, IsWindow, IsZoomed,
    SW_RESTORE, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow,
};

use crate::Window;

use super::window_mode::resize_windowed_client;

// 窗口激活并置顶工具函数（强化版本，用于需要前台的物理输入方式）
// 物理输入前使用。
pub(crate) fn ensure_foreground_and_topmost(window: Window) -> anyhow::Result<()> {
    let hwnd = window.raw;
    if hwnd.is_invalid() {
        bail!("hwnd is invalid");
    }

    // 如果窗口不在前台，先将其置顶
    if hwnd != unsafe { GetForegroundWindow() } {
        // 将窗口移到 Z 序顶部
        unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            )
        }?;

        // 尝试设置为前台窗口
        unsafe { SetForegroundWindow(hwnd) }.ok()?;
    }

    Ok(())
}

/// 若窗口处于最小化状态则恢复（取消最小化）。
///
/// 连接游戏时调用：窗口最小化时 [`ensure_window_on_screen`] 会跳过调整，
/// 需先恢复窗口才能正确获取并调整客户区。
pub(crate) fn restore_window_if_minimized(window: Window) -> anyhow::Result<()> {
    let hwnd = window.raw;
    if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        bail!("Invalid window handle");
    }

    if unsafe { IsIconic(hwnd) }.as_bool() {
        unsafe { ShowWindow(hwnd, SW_RESTORE) }.ok()?;
    }

    Ok(())
}

/// Ensure the window's client area is fully visible on the monitor.
/// If the window extends beyond the monitor bounds, move it back.
/// If the client area is larger than the monitor, resize it unless its size must be preserved.
pub(crate) fn ensure_window_on_screen(window: Window, preserve_size: bool) -> anyhow::Result<()> {
    let hwnd = window.raw;
    if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        bail!("Invalid window handle");
    }

    // Don't adjust maximized or minimized windows
    if unsafe { IsZoomed(hwnd) }.as_bool() || unsafe { IsIconic(hwnd) }.as_bool() {
        return Ok(());
    }

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_invalid() {
        bail!("Failed to get monitor for window");
    }

    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut mi) }.ok()?;

    let monitor_rect = mi.rcWork;
    let monitor_w = monitor_rect.right - monitor_rect.left;
    let monitor_h = monitor_rect.bottom - monitor_rect.top;

    // 客户区大于工作区时由窗口尺寸调整统一处理；保留尺寸模式只移动窗口。
    let mut client_rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client_rect) }?;
    let client_w = client_rect.right - client_rect.left;
    let client_h = client_rect.bottom - client_rect.top;
    if !preserve_size && (client_w > monitor_w || client_h > monitor_h) {
        resize_windowed_client(window, client_w.min(monitor_w), client_h.min(monitor_h))?;
    }

    // Move the client area back into the monitor bounds without resizing again.
    let mut window_rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut window_rect) }?;
    unsafe { GetClientRect(hwnd, &mut client_rect) }?;
    let mut client_origin = POINT::default();
    unsafe { ClientToScreen(hwnd, &mut client_origin) }.ok()?;

    let frame_left = client_origin.x - window_rect.left;
    let frame_top = client_origin.y - window_rect.top;

    let client_w = client_rect.right - client_rect.left;
    let client_h = client_rect.bottom - client_rect.top;

    let mut new_client_x = client_origin.x;
    let mut new_client_y = client_origin.y;

    // Adjust right/bottom first
    if new_client_x + client_w > monitor_rect.right {
        new_client_x = monitor_rect.right - client_w;
    }
    if new_client_y + client_h > monitor_rect.bottom {
        new_client_y = monitor_rect.bottom - client_h;
    }

    // Then adjust left/top (takes priority)
    if new_client_x < monitor_rect.left {
        new_client_x = monitor_rect.left;
    }
    if new_client_y < monitor_rect.top {
        new_client_y = monitor_rect.top;
    }

    if new_client_x == client_origin.x && new_client_y == client_origin.y {
        return Ok(());
    }

    // Convert client coordinates back to window coordinates
    let new_window_x = new_client_x - frame_left;
    let new_window_y = new_client_y - frame_top;

    unsafe {
        SetWindowPos(
            hwnd,
            None,
            new_window_x,
            new_window_y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
    }?;

    Ok(())
}
