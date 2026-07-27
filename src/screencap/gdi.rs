use anyhow::{Context, Result, bail};
use image::RgbaImage;
use scopeguard::defer;
use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetBitmapBits,
    GetDC, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use crate::screencap::base::ScreencapBase;

pub struct GdiScreencap {
    hwnd: HWND,
}

impl GdiScreencap {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn screencap(&mut self) -> Result<RgbaImage> {
        if self.hwnd.is_invalid() {
            bail!("hwnd is nullptr");
        }

        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect)?;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            bail!("Invalid window size width={} height={}", width, height);
        }

        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        if hdc.is_invalid() {
            bail!("GetDC failed, error code: {:?}", unsafe { GetLastError() });
        }
        defer! {
            unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        }

        let mem_dc = unsafe { CreateCompatibleDC(Some(hdc)) };
        if mem_dc.is_invalid() {
            bail!("CreateCompatibleDC failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteDC(mem_dc) };
        }

        let bitmap = unsafe { CreateCompatibleBitmap(hdc, width, height) };
        if bitmap.is_invalid() {
            bail!("CreateCompatibleBitmap failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteObject(bitmap.into()) };
        }

        let old_obj = unsafe { SelectObject(mem_dc, bitmap.into()) };
        if old_obj.is_invalid() {
            bail!("SelectObject failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            unsafe { SelectObject(mem_dc, old_obj) };
        }

        unsafe { BitBlt(mem_dc, 0, 0, width, height, Some(hdc), 0, 0, SRCCOPY) }?;

        let mut mat = vec![0u8; (width * height * 4) as usize];
        if unsafe { GetBitmapBits(bitmap, width * height * 4, mat.as_mut_ptr() as *mut _) } == 0 {
            bail!("GetBitmapBits failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }

        RgbaImage::from_raw_bgra(width as u32, height as u32, mat).context("从原始数据创建图像失败")
    }
}

impl ScreencapBase for GdiScreencap {
    fn new(hwnd: HWND) -> Self {
        Self::new(hwnd)
    }

    fn screencap(&mut self) -> Result<RgbaImage> {
        self.screencap()
    }
}
