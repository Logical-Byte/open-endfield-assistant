use anyhow::Result;
use windows::Win32::Foundation::HWND;

pub trait ScreencapBase<T> {
    fn new(hwnd: HWND) -> Self;

    fn screencap(&mut self) -> Result<T>;
}
