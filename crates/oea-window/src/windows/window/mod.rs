mod get_window;
pub(crate) mod hdr;
#[allow(dead_code)]
mod window_info;
mod window_mode;
mod window_operation;

pub(crate) use get_window::search;
pub(crate) use window_info::get_client_rect;
pub(crate) use window_mode::{ensure_windowed, resize_windowed_client};
pub(crate) use window_operation::{
    ensure_foreground_and_topmost, ensure_window_on_screen, restore_window_if_minimized,
};

use crate::Window;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub(crate) fn is_foreground(window: Window) -> bool {
    (unsafe { GetForegroundWindow() }) == window.raw
}
