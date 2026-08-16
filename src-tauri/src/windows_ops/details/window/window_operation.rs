use anyhow::{Result, bail};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetForegroundWindow, GetWindowRect, HWND_TOP, IsIconic, IsWindow, IsZoomed,
    SW_RESTORE, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow,
};

use crate::windows_ops::WindowHandle;

/// 设置当前线程的 DPI 感知上下文为 Per Monitor v2
pub fn set_thread_dpi_awareness_context() -> DPI_AWARENESS_CONTEXT {
    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
}

// 窗口激活并置顶工具函数（强化版本，用于需要前台的物理输入方式）
// 用于 LegacyEventInput 和 SeizeInput，因为它们使用 SendInput/mouse_event 等物理输入 API
pub fn ensure_foreground_and_topmost(window: WindowHandle) -> Result<()> {
    let hwnd = window.0;
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
pub fn restore_window_if_minimized(window: WindowHandle) -> Result<()> {
    let hwnd = window.0;
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
/// If the client area is larger than the monitor, resize the window.
pub fn ensure_window_on_screen(window: WindowHandle) -> Result<()> {
    let hwnd = window.0;
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
