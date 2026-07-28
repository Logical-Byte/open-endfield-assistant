use anyhow::Result;
use image::{RgbaImage, imageops};
use windows::Win32::Foundation::HWND;

use crate::utils::region::Region2D;

pub trait ScreencapBase {
    fn new(hwnd: HWND) -> Self;

    fn screencap(&mut self) -> Result<RgbaImage>;

    fn screencap_region(&mut self, relative_region: Region2D<u32>) -> Result<RgbaImage> {
        let image = self.screencap()?;
        let cropped = imageops::crop_imm(
            &image,
            relative_region.x0(),
            relative_region.y0(),
            relative_region.width(),
            relative_region.height(),
        )
        .to_image();
        Ok(cropped)
    }
}
