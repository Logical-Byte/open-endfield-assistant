use anyhow::{Context, Result, bail};
use image::{ImageBuffer, Rgba};
use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetBitmapBits,
    GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use crate::screencap::base::ScreencapBase;

pub struct GdiScreencap {
    hwnd: HWND,
}

impl ScreencapBase<ImageBuffer<Rgba<u8>, Vec<u8>>> for GdiScreencap {
    fn new(hwnd: HWND) -> Self {
        Self::new(hwnd)
    }

    fn screencap(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.screencap()
    }
}

impl GdiScreencap {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn screencap(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        if self.hwnd.is_invalid() {
            bail!("hwnd_ is nullptr");
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

        let hdc: HDC = unsafe { GetDC(Some(self.hwnd)) };
        if hdc.is_invalid() {
            bail!("GetDC failed, error code: {:?}", unsafe { GetLastError() });
        }
        let _hdc_guard = DcGuard(self.hwnd, hdc);

        let mem_dc: HDC = unsafe { CreateCompatibleDC(Some(hdc)) };
        if mem_dc.is_invalid() {
            bail!("CreateCompatibleDC failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        let _mem_dc_guard = MemDcGuard(mem_dc);

        let bitmap: HBITMAP = unsafe { CreateCompatibleBitmap(hdc, width, height) };
        if bitmap.is_invalid() {
            bail!("CreateCompatibleBitmap failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        let _bitmap_guard = BitmapGuard(bitmap);

        let old_obj: HGDIOBJ = unsafe { SelectObject(mem_dc, bitmap.into()) };
        if old_obj.is_invalid() {
            bail!("SelectObject failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        let _select_guard = SelectGuard(mem_dc, old_obj);

        unsafe { BitBlt(mem_dc, 0, 0, width, height, Some(hdc), 0, 0, SRCCOPY)? };

        let mut mat = vec![0u8; (width * height * 4) as usize];
        if unsafe { GetBitmapBits(bitmap, width * height * 4, mat.as_mut_ptr() as *mut _) } == 0 {
            bail!("GetBitmapBits failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }

        // bgra_to_bgr: 交换 B 和 R 通道（对应 C++ 的 bgra_to_bgr）
        for chunk in mat.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        ImageBuffer::from_raw(width as u32, height as u32, mat).context("从原始数据创建图像失败")
    }
}

struct DcGuard(HWND, HDC);
impl Drop for DcGuard {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(Some(self.0), self.1);
        }
    }
}

struct MemDcGuard(HDC);
impl Drop for MemDcGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

struct BitmapGuard(HBITMAP);
impl Drop for BitmapGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

struct SelectGuard(HDC, HGDIOBJ);
impl Drop for SelectGuard {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.0, self.1);
        }
    }
}
