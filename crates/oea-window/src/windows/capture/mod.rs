//! 原始截图 implementation。生产使用 `PrintWindow`，其余实现保留供实验。

use super::geometry::Region;
use image::{RgbaImage, imageops};

#[allow(dead_code)]
mod desktop_dup;
#[allow(dead_code)]
mod desktop_dup_window;
#[allow(dead_code)]
mod frame_pool;
#[allow(dead_code)]
mod gdi;
mod print_window;
#[allow(dead_code)]
mod screen_dc;

pub(crate) use print_window::PrintWindowState;

#[allow(dead_code)]
trait ScreencapBase: Send {
    fn screencap(&mut self) -> anyhow::Result<RgbaImage>;

    fn screencap_region(&mut self, region: Region) -> anyhow::Result<RgbaImage> {
        let image = self.screencap()?;
        Ok(imageops::crop_imm(
            &image,
            region.x0() as u32,
            region.y0() as u32,
            region.width() as u32,
            region.height() as u32,
        )
        .to_image())
    }
}
