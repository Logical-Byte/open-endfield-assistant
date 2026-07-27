use anyhow::{Context, Result, bail};
use image::RgbaImage;
use scopeguard::defer;
use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PW_CLIENTONLY, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::base::ScreencapBase;
use crate::utils::point::Region2D;

// PW_RENDERFULLCONTENT (0x2): 捕获非最小化后台窗口
const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(0x2_u32);

pub struct PrintWindowScreencap {
    hwnd: HWND,
}

impl PrintWindowScreencap {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn screencap(&mut self) -> Result<RgbaImage> {
        if self.hwnd.is_invalid() {
            bail!("hwnd is nullptr");
        }

        // 确定要捕获的区域大小
        // 使用 PW_CLIENTONLY 标志，只获取客户端区域（不含窗口边框）
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
            let _ = unsafe { SelectObject(mem_dc, old_obj) };
        }

        // 使用 PrintWindow 捕获窗口内容
        // 使用 PW_CLIENTONLY | PW_RENDERFULLCONTENT 标志:
        // - PW_CLIENTONLY (0x1)：只获取客户端区域
        // - PW_RENDERFULLCONTENT (0x2)：捕获非最小化后台窗口
        let n_flags = PRINT_WINDOW_FLAGS(PW_CLIENTONLY.0 | PW_RENDERFULLCONTENT.0);
        unsafe { PrintWindow(self.hwnd, mem_dc, n_flags) }.ok()?;

        // 使用 GetDIBits 将位图一致转换为 32bpp BGRA
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut mat = vec![0u8; (width * height * 4) as usize];
        if unsafe {
            GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                Some(mat.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            )
        } == 0
        {
            bail!("GetDIBits failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }

        RgbaImage::from_raw_bgra(width as u32, height as u32, mat).context("从原始数据创建图像失败")
    }

    pub fn screencap_region(&mut self, relative_region: Region2D<u32>) -> Result<RgbaImage> {
        if self.hwnd.is_invalid() {
            bail!("hwnd is nullptr");
        }

        let x1 = relative_region.x1();
        let y1 = relative_region.y1();
        let width = relative_region.width();
        let height = relative_region.height();

        if width <= 0 || height <= 0 {
            bail!("Invalid region size width={} height={}", width, height,);
        }

        // 获取整个客户区大小
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect)?;
        }

        let full_w = rect.right - rect.left;
        let full_h = rect.bottom - rect.top;

        if full_w <= 0 || full_h <= 0 {
            bail!("Invalid window size width={} height={}", full_w, full_h);
        }

        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        if hdc.is_invalid() {
            bail!("GetDC failed, error code: {:?}", unsafe { GetLastError() });
        }
        defer! {
            unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        }

        // 创建全窗口大小的内存 DC，用 PrintWindow 渲染
        let full_mem_dc = unsafe { CreateCompatibleDC(Some(hdc)) };
        if full_mem_dc.is_invalid() {
            bail!("CreateCompatibleDC failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteDC(full_mem_dc) };
        }

        let full_bitmap = unsafe { CreateCompatibleBitmap(hdc, x1 as i32, y1 as i32) };
        if full_bitmap.is_invalid() {
            bail!("CreateCompatibleBitmap failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteObject(full_bitmap.into()) };
        }

        let old_full_obj = unsafe { SelectObject(full_mem_dc, full_bitmap.into()) };
        if old_full_obj.is_invalid() {
            bail!("SelectObject failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { SelectObject(full_mem_dc, old_full_obj) };
        }

        // 使用 PrintWindow 捕获整个窗口内容
        let n_flags = PRINT_WINDOW_FLAGS(PW_CLIENTONLY.0 | PW_RENDERFULLCONTENT.0);
        unsafe { PrintWindow(self.hwnd, full_mem_dc, n_flags) }.ok()?;

        // 创建区域大小的内存 DC，用 BitBlt 从全窗口 DC 中截取目标区域
        let region_mem_dc = unsafe { CreateCompatibleDC(Some(hdc)) };
        if region_mem_dc.is_invalid() {
            bail!(
                "CreateCompatibleDC(region) failed, error code: {:?}",
                unsafe { GetLastError() }
            );
        }
        defer! {
            let _ = unsafe { DeleteDC(region_mem_dc) };
        }

        let region_bitmap = unsafe { CreateCompatibleBitmap(hdc, width as i32, height as i32) };
        if region_bitmap.is_invalid() {
            bail!(
                "CreateCompatibleBitmap(region) failed, error code: {:?}",
                unsafe { GetLastError() }
            );
        }
        defer! {
            let _ = unsafe { DeleteObject(region_bitmap.into()) };
        }

        let old_region_obj = unsafe { SelectObject(region_mem_dc, region_bitmap.into()) };
        if old_region_obj.is_invalid() {
            bail!("SelectObject(region) failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { SelectObject(region_mem_dc, old_region_obj) };
        }

        // 从全窗口 DC 中复制指定区域到区域 DC
        unsafe {
            BitBlt(
                region_mem_dc,
                0,
                0,
                width as i32,
                height as i32,
                Some(full_mem_dc),
                relative_region.x0() as i32,
                relative_region.y0() as i32,
                SRCCOPY,
            )?;
        }

        // 使用 GetDIBits 从区域位图读取像素数据
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let pixel_count = (width * height) as usize;
        let mut mat = vec![0u8; pixel_count * 4];
        if unsafe {
            GetDIBits(
                region_mem_dc,
                region_bitmap,
                0,
                height,
                Some(mat.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            )
        } == 0
        {
            bail!("GetDIBits failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }

        RgbaImage::from_raw_bgra(width, height, mat).context("从原始数据创建图像失败")
    }
}

impl ScreencapBase for PrintWindowScreencap {
    fn new(hwnd: HWND) -> Self {
        Self::new(hwnd)
    }

    fn screencap(&mut self) -> Result<RgbaImage> {
        self.screencap()
    }

    fn screencap_region(&mut self, relative_region: Region2D<u32>) -> Result<RgbaImage> {
        self.screencap_region(relative_region)
    }
}
