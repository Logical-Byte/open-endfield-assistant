//! 在已校验的图片区域内统计亮度和颜色，不解释具体界面的状态。

use image::{GenericImageView, Pixel};

use super::ImageRegion;

/// 返回图片指定区域的平均亮度，范围为 `0..=255`。
///
/// 使用 `image` 的 RGB 转灰度权重，忽略 alpha。
pub(crate) fn mean_luma<I>(region: &ImageRegion<'_, I>) -> f32
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
{
    let view = region.view();
    let total: u64 = view
        .pixels()
        .map(|(_, _, pixel)| u64::from(pixel.to_luma().0[0]))
        .sum();
    let pixel_count = u64::from(view.width()) * u64::from(view.height());
    total as f32 / pixel_count as f32
}

#[cfg(test)]
mod tests {
    use image::{GrayImage, Luma, Rgba, RgbaImage};

    use crate::utils::region::Region2D;

    use super::*;

    #[test]
    fn measures_only_the_requested_region_and_ignores_alpha() {
        let mut image = RgbaImage::from_pixel(4, 3, Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 1, Rgba([20, 40, 60, 0]));
        image.put_pixel(2, 1, Rgba([40, 60, 80, 255]));
        let region = ImageRegion::new(&image, Region2D::from_ltwh(1, 1, 2, 1)).unwrap();

        assert_eq!(mean_luma(&region), 47.0);
    }

    #[test]
    fn preserves_fractional_brightness_of_grayscale_samples() {
        let image = GrayImage::from_fn(2, 1, |x, _| Luma([200 + x as u8]));
        assert_eq!(
            mean_luma(&ImageRegion::new(&image, Region2D::from_ltwh(0, 0, 2, 1)).unwrap()),
            200.5
        );
    }
}
