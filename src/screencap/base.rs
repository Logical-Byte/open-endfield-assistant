use anyhow::Result;
use image::{RgbaImage, imageops};
use windows::Win32::Foundation::HWND;

use crate::utils::region::Region2D;

pub trait ScreencapBase {
    fn new(hwnd: HWND) -> Self
    where
        Self: Sized;

    fn screencap(&mut self) -> Result<RgbaImage>;

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
