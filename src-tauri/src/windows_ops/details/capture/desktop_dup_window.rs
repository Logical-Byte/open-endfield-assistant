use anyhow::{Result, anyhow, bail};
use image::RgbaImage;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::base::ScreencapBase;
use super::desktop_dup::DesktopDupScreencap;
use crate::windows_ops::WindowHandle;

/// 基于 Desktop Duplication 的窗口截图器
/// 先截取全屏，再根据窗口客户区坐标裁剪
pub struct DesktopDupWindowScreencap {
    hwnd: HWND,
    inner: DesktopDupScreencap,
}

impl DesktopDupWindowScreencap {
    pub fn new(window: WindowHandle) -> Self {
        Self {
            hwnd: window.0,
            inner: DesktopDupScreencap::new(window),
        }
    }

    pub fn screencap(&mut self) -> Result<RgbaImage> {
        if self.hwnd.is_invalid() {
            bail!("hwnd is nullptr");
        }

        // 调用 DesktopDupScreencap 获取全屏截图
        let img = self.inner.screencap()?;

        // 获取窗口客户区在屏幕上的位置（相对于整个虚拟桌面）
        let client_rect_screen = self.get_window_client_rect_screen()?;

        // 获取当前输出（显示器）的桌面坐标
        let output_desktop = self.inner.output_desktop_coordinates()?;

        // 将窗口坐标转换为相对于该显示器的坐标
        let client_width = client_rect_screen.right - client_rect_screen.left;
        let client_height = client_rect_screen.bottom - client_rect_screen.top;
        let crop_x = client_rect_screen.left - output_desktop.left;
        let crop_y = client_rect_screen.top - output_desktop.top;

        // 检查裁剪区域是否在图像范围内
        if crop_x < 0
            || crop_y < 0
            || crop_x + client_width > img.width() as i32
            || crop_y + client_height > img.height() as i32
        {
            bail!(
                "Client rect out of bounds crop_x={} crop_y={} client_width={} client_height={} img_width={} img_height={} output_left={} output_top={}",
                crop_x,
                crop_y,
                client_width,
                client_height,
                img.width(),
                img.height(),
                output_desktop.left,
                output_desktop.top,
            );
        }

        // 裁剪出窗口客户区
        let cropped = image::imageops::crop_imm(
            &img,
            crop_x as u32,
            crop_y as u32,
            client_width as u32,
            client_height as u32,
        )
        .to_image();

        Ok(cropped)
    }

    /// 获取窗口客户区在屏幕上的位置（对应 C++ get_window_client_rect_screen）
    fn get_window_client_rect_screen(&self) -> Result<RECT> {
        let mut client_rect = RECT::default();

        // 获取窗口客户区（相对于窗口）
        unsafe { GetClientRect(self.hwnd, &mut client_rect) }
            .map_err(|e| anyhow!("GetClientRect failed: {:?}", e))?;

        if client_rect.right <= client_rect.left || client_rect.bottom <= client_rect.top {
            bail!(
                "Invalid client rect left={} top={} right={} bottom={}",
                client_rect.left,
                client_rect.top,
                client_rect.right,
                client_rect.bottom,
            );
        }

        // 将客户区左上角转换为屏幕坐标（相对于整个虚拟桌面）
        let mut client_top_left = windows::Win32::Foundation::POINT {
            x: client_rect.left,
            y: client_rect.top,
        };
        unsafe { ClientToScreen(self.hwnd, &mut client_top_left) }
            .ok()
            .map_err(|e| anyhow!("ClientToScreen failed: {:?}", e))?;

        // 计算客户区在屏幕上的位置
        let client_width = client_rect.right - client_rect.left;
        let client_height = client_rect.bottom - client_rect.top;

        Ok(RECT {
            left: client_top_left.x,
            top: client_top_left.y,
            right: client_top_left.x + client_width,
            bottom: client_top_left.y + client_height,
        })
    }
}

// Win32 句柄（HWND 等）跨线程传递安全（访问时由调用方串行化）。
unsafe impl Send for DesktopDupWindowScreencap {}

impl ScreencapBase for DesktopDupWindowScreencap {
    fn new(window: WindowHandle) -> Self {
        Self::new(window)
    }

    fn screencap(&mut self) -> Result<RgbaImage> {
        self.screencap()
    }
}
