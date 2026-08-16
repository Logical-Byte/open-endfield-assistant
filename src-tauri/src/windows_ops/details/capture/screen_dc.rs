use anyhow::{Context, Result, bail};
use image::RgbaImage;
use scopeguard::defer;
use windows::Win32::Foundation::{GetLastError, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    GetBitmapBits, GetDC, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::base::ScreencapBase;
use crate::utils::region::Region2D;

pub struct ScreenDCScreencap {
    hwnd: HWND,
}

impl ScreenDCScreencap {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn screencap(&mut self) -> Result<RgbaImage> {
        // 获取客户区大小
        let mut client_rect = RECT::default();
        unsafe { GetClientRect(self.hwnd, &mut client_rect) }?;

        self.screencap_region(Region2D::from(client_rect))
    }

    pub fn screencap_region(&mut self, region: Region2D<i32>) -> Result<RgbaImage> {
        let mut p0 = POINT::from(region.p0());
        unsafe { ClientToScreen(self.hwnd, &mut p0) }.ok()?;

        let mut p1 = POINT::from(region.p1());
        unsafe { ClientToScreen(self.hwnd, &mut p1) }.ok()?;

        let region = Region2D::from_points(p0.into(), p1.into());

        self.screencap_screen_region(region)
    }

    fn screencap_screen_region(&self, region: Region2D<i32>) -> Result<RgbaImage> {
        if self.hwnd.is_invalid() {
            bail!("hwnd is nullptr");
        }

        let x0 = region.x0();
        let y0 = region.y0();
        let width = region.width();
        let height = region.height();

        if width <= 0 || height <= 0 {
            bail!("Invalid window size width={} height={}", width, height);
        }

        let screen_dc = unsafe { GetDC(None) };
        if screen_dc.is_invalid() {
            bail!("GetDC(screen) failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            unsafe { ReleaseDC(None, screen_dc) };
        }

        let mem_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
        if mem_dc.is_invalid() {
            bail!("CreateCompatibleDC failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteDC(mem_dc) };
        }

        let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
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

        // 从屏幕 DC 复制客户区内容
        unsafe {
            BitBlt(
                mem_dc,
                0,
                0,
                width,
                height,
                Some(screen_dc),
                x0,
                y0,
                SRCCOPY,
            )?;
        }

        let mut mat = vec![0u8; (width * height * 4) as usize];
        if unsafe { GetBitmapBits(bitmap, width * height * 4, mat.as_mut_ptr() as *mut _) } == 0 {
            bail!("GetBitmapBits failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }

        RgbaImage::from_raw_bgra(width as u32, height as u32, mat).context("从原始数据创建图像失败")
    }
}

// Win32 句柄（HWND 等）跨线程传递安全（访问时由调用方串行化）。
unsafe impl Send for ScreenDCScreencap {}

impl ScreencapBase for ScreenDCScreencap {
    fn new(hwnd: HWND) -> Self {
        Self::new(hwnd)
    }

    fn screencap(&mut self) -> Result<RgbaImage> {
        Self::screencap(self)
    }

    fn screencap_region(&mut self, relative_region: Region2D<i32>) -> Result<RgbaImage> {
        Self::screencap_region(self, relative_region)
    }
}
