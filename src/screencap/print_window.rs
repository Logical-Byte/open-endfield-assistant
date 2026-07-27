use anyhow::{Context, Result, bail};
use image::{ImageBuffer, Rgba};
use scopeguard::defer;
use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP, HDC, HGDIOBJ, ReleaseDC,
    SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PW_CLIENTONLY, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::base::ScreencapBase;

// PW_RENDERFULLCONTENT (0x2): 捕获非最小化后台窗口
const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(0x2_u32);

pub struct PrintWindowScreencap {
    hwnd: HWND,
}
impl ScreencapBase<ImageBuffer<Rgba<u8>, Vec<u8>>> for PrintWindowScreencap {
    fn new(hwnd: HWND) -> Self {
        Self::new(hwnd)
    }

    fn screencap(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.screencap()
    }
}

impl PrintWindowScreencap {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn screencap(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        if self.hwnd.is_invalid() {
            bail!("hwnd_ is nullptr");
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

        let hdc: HDC = unsafe { GetDC(Some(self.hwnd)) };
        if hdc.is_invalid() {
            bail!("GetDC failed, error code: {:?}", unsafe { GetLastError() });
        }
        defer! {
            unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        }

        let mem_dc: HDC = unsafe { CreateCompatibleDC(Some(hdc)) };
        if mem_dc.is_invalid() {
            bail!("CreateCompatibleDC failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteDC(mem_dc) };
        }

        let bitmap: HBITMAP = unsafe { CreateCompatibleBitmap(hdc, width, height) };
        if bitmap.is_invalid() {
            bail!("CreateCompatibleBitmap failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }
        defer! {
            let _ = unsafe { DeleteObject(bitmap.into()) };
        }

        let old_obj: HGDIOBJ = unsafe { SelectObject(mem_dc, bitmap.into()) };
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
        if !unsafe { PrintWindow(self.hwnd, mem_dc, n_flags) }.as_bool() {
            bail!("PrintWindow failed, error code: {:?}", unsafe {
                GetLastError()
            });
        }

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

        // bgra_to_rgba: 交换 B 和 R 通道
        for chunk in mat.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        ImageBuffer::from_raw(width as u32, height as u32, mat).context("从原始数据创建图像失败")
    }
}
