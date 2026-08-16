//! 窗口截图接口。

use anyhow::Result;
use image::{RgbaImage, imageops};

use crate::utils::region::Region2D;

use super::{WindowHandle, details};

/// 可替换的窗口截图能力。
pub trait ScreencapBase: Send {
    /// 捕获完整窗口客户区。
    fn screencap(&mut self) -> Result<RgbaImage>;

    /// 捕获相对于窗口客户区的区域。
    fn screencap_region(&mut self, relative_region: Region2D<i32>) -> Result<RgbaImage> {
        let image = self.screencap()?;
        let cropped = imageops::crop_imm(
            &image,
            relative_region.x0() as u32,
            relative_region.y0() as u32,
            relative_region.width() as u32,
            relative_region.height() as u32,
        )
        .to_image();
        Ok(cropped)
    }
}

/// 使用 `PrintWindow` 捕获窗口客户区的截图器。
pub struct PrintWindowScreencap {
    state: details::capture::PrintWindowState,
}

impl PrintWindowScreencap {
    /// 绑定要捕获的窗口。
    pub fn new(window: WindowHandle) -> Self {
        Self {
            state: details::capture::PrintWindowState::new(window),
        }
    }

    /// 捕获完整窗口客户区。
    pub fn screencap(&mut self) -> Result<RgbaImage> {
        self.state.screencap()
    }
}

// 原生窗口句柄跨线程传递安全；截图器由调用方串行访问。
unsafe impl Send for PrintWindowScreencap {}

impl ScreencapBase for PrintWindowScreencap {
    fn screencap(&mut self) -> Result<RgbaImage> {
        self.screencap()
    }

    fn screencap_region(&mut self, relative_region: Region2D<i32>) -> Result<RgbaImage> {
        self.state.screencap_region(relative_region)
    }
}
