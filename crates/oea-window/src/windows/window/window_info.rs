use ::windows::Win32::Foundation::{POINT, RECT};
use ::windows::Win32::Graphics::Gdi::ClientToScreen;
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetClassNameW, GetClientRect, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowTextW, WS_POPUP,
};
use anyhow::bail;

use crate::Point;
use crate::Window;
use crate::windows::geometry::Region;

fn get_window_title(window: Window) -> anyhow::Result<String> {
    let hwnd = window.raw;
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length == 0 {
        bail!("Failed to get window title");
    }

    let mut buffer = vec![0u16; (length + 1) as usize];
    let read_length = unsafe { GetWindowTextW(hwnd, &mut buffer) };

    let title = String::from_utf16_lossy(&buffer[..read_length as usize]);
    Ok(title)
}

pub(crate) fn get_client_rect(window: Window) -> anyhow::Result<Region> {
    let hwnd = window.raw;
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) }?;
    Ok(rect.into())
}

fn client_to_screen(window: Window, point: Point) -> anyhow::Result<Point> {
    let hwnd = window.raw;
    let mut point = POINT::from(point);
    unsafe { ClientToScreen(hwnd, &mut point) }.ok()?;
    Ok(point.into())
}

fn get_window_class_name(window: Window) -> anyhow::Result<String> {
    let hwnd = window.raw;
    let mut buffer = vec![0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if length == 0 {
        bail!("Failed to get window class name");
    }

    let class_name = String::from_utf16_lossy(&buffer[..length as usize]);
    Ok(class_name)
}

fn is_fullscreen(window: Window) -> bool {
    let hwnd = window.raw;
    (unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32) & WS_POPUP.0 != 0
}
