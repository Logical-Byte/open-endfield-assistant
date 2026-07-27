use anyhow::{Result, bail};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetClassNameW, GetClientRect, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, IsIconic, IsWindow, IsZoomed, SWP_ASYNCWINDOWPOS,
    SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos, WS_POPUP,
};

use crate::utils::point::{Point2D, Region2D};

pub fn get_window_title(hwnd: HWND) -> Result<String> {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length == 0 {
        bail!("Failed to get window title");
    }

    let mut buffer = vec![0u16; (length + 1) as usize];
    let read_length = unsafe { GetWindowTextW(hwnd, &mut buffer) };

    let title = String::from_utf16_lossy(&buffer[..read_length as usize]);
    Ok(title)
}

pub fn get_client_rect(hwnd: HWND) -> Result<Region2D<i32>> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) }?;
    Ok(rect.into())
}

pub fn client_to_screen(hwnd: HWND, x: i32, y: i32) -> Result<Point2D<i32>> {
    let mut point = POINT { x, y };
    unsafe { ClientToScreen(hwnd, &mut point) }.ok()?;
    Ok(point.into())
}

pub fn get_window_class_name(hwnd: HWND) -> Result<String> {
    let mut buffer = vec![0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if length == 0 {
        bail!("Failed to get window class name");
    }

    let class_name = String::from_utf16_lossy(&buffer[..length as usize]);
    Ok(class_name)
}

pub fn is_fullscreen(hwnd: HWND) -> bool {
    (unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32) & WS_POPUP.0 != 0
}

/// Ensure the window's client area is fully visible on the monitor.
/// If the window extends beyond the monitor bounds, move it back.
/// If the client area is larger than the monitor, resize the window.
pub fn ensure_window_on_screen(hwnd: HWND) -> Result<()> {
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

    // Get window rect and client rect to calculate frame sizes
    let mut window_rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut window_rect) }?;

    let mut client_rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client_rect) }?;

    let mut client_origin = POINT::default();
    unsafe { ClientToScreen(hwnd, &mut client_origin) }.ok()?;

    let frame_left = client_origin.x - window_rect.left;
    let frame_top = client_origin.y - window_rect.top;
    let frame_right = window_rect.right - client_origin.x - client_rect.right;
    let frame_bottom = window_rect.bottom - client_origin.y - client_rect.bottom;

    let client_w = client_rect.right - client_rect.left;
    let client_h = client_rect.bottom - client_rect.top;

    let mut need_change = false;

    // If client area is larger than monitor, cap to monitor size
    let mut new_client_w = client_w;
    let mut new_client_h = client_h;

    if new_client_w > monitor_w {
        new_client_w = monitor_w;
        need_change = true;
    }
    if new_client_h > monitor_h {
        new_client_h = monitor_h;
        need_change = true;
    }

    // Calculate desired client area position
    let mut new_client_x = client_origin.x;
    let mut new_client_y = client_origin.y;

    // Adjust right/bottom first
    if new_client_x + new_client_w > monitor_rect.right {
        new_client_x = monitor_rect.right - new_client_w;
        need_change = true;
    }
    if new_client_y + new_client_h > monitor_rect.bottom {
        new_client_y = monitor_rect.bottom - new_client_h;
        need_change = true;
    }

    // Then adjust left/top (takes priority)
    if new_client_x < monitor_rect.left {
        new_client_x = monitor_rect.left;
        need_change = true;
    }
    if new_client_y < monitor_rect.top {
        new_client_y = monitor_rect.top;
        need_change = true;
    }

    if !need_change {
        return Ok(());
    }

    // Convert client coordinates back to window coordinates
    let new_window_x = new_client_x - frame_left;
    let new_window_y = new_client_y - frame_top;
    let new_window_w = new_client_w + frame_left + frame_right;
    let new_window_h = new_client_h + frame_top + frame_bottom;

    // SWP_ASYNCWINDOWPOS: avoid blocking if the target window's thread is busy/hung
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            new_window_x,
            new_window_y,
            new_window_w,
            new_window_h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
    }?;

    Ok(())
}
