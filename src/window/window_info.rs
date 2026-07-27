use anyhow::{Result, bail};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetClassNameW, GetClientRect, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, WS_POPUP,
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

pub fn client_to_screen(hwnd: HWND, point: Point2D<i32>) -> Result<Point2D<i32>> {
    let mut point = POINT::from(point);
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
