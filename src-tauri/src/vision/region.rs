use image::{GenericImageView, Pixel, imageops};

use crate::utils::region::Region2D;

/// 返回图片指定区域的平均亮度，空区域没有平均值。
pub fn mean_luma<I>(image: &I, region: Region2D<u32>) -> Option<f32>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
{
    let pixel_count = u64::from(region.width()) * u64::from(region.height());
    if pixel_count == 0 {
        return None;
    }

    let cropped = imageops::crop_imm(
        image,
        region.x0(),
        region.y0(),
        region.width(),
        region.height(),
    );
    let gray = imageops::grayscale(&*cropped);
    let total: u64 = gray.pixels().map(|pixel| u64::from(pixel.0[0])).sum();

    Some(total as f32 / pixel_count as f32)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use crate::utils::region::Region2D;

    use super::mean_luma;

    #[test]
    fn computes_mean_luma_in_region() {
        let image = RgbaImage::from_fn(3, 1, |x, _| match x {
            0 => Rgba([0, 0, 0, 255]),
            1 => Rgba([255, 255, 255, 255]),
            _ => Rgba([255, 0, 0, 255]),
        });

        let mean = mean_luma(&image, Region2D::from_ltwh(0, 0, 2, 1));

        assert_eq!(mean, Some(127.5));
    }
}
