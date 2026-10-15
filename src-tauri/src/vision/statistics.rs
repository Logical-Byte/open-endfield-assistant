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

/// 分别计算区域内 R、G、B 通道的算术平均值，返回 `[R, G, B]`。
///
/// 数组中的三个值均为 `f32`，范围为 `0..=255`，保留平均值的小数部分。忽略 alpha。
/// 调用方需要 HSV 时，可将返回的 RGB 数组传给 `vision::color::Hsv::from_rgb`。
#[allow(dead_code, reason = "通用识别 primitive，供后续工作流调用")]
pub(crate) fn mean_rgb<I>(region: &ImageRegion<'_, I>) -> [f32; 3]
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
{
    let view = region.view();
    let mut total = [0_u64; 3];
    for (_, _, pixel) in view.pixels() {
        for (sum, channel) in total.iter_mut().zip(pixel.to_rgb().0) {
            *sum += u64::from(channel);
        }
    }
    let pixel_count = u64::from(view.width()) * u64::from(view.height());
    total.map(|sum| sum as f32 / pixel_count as f32)
}

/// 统计区域内 RGB 三个通道都严格大于对应下限的像素数，忽略 alpha。
///
/// 可用于观察少量亮色像素，避免平均颜色掩盖局部特征。
/// 像素数量的业务含义由调用方解释。
#[allow(dead_code, reason = "通用识别 primitive，供后续工作流调用")]
pub(crate) fn count_rgb_above<I>(region: &ImageRegion<'_, I>, thresholds: [u8; 3]) -> u64
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
{
    region
        .view()
        .pixels()
        .filter(|(_, _, pixel)| {
            pixel
                .to_rgb()
                .0
                .into_iter()
                .zip(thresholds)
                .all(|(channel, threshold)| channel > threshold)
        })
        .count() as u64
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

        assert_eq!(mean_rgb(&region), [30.0, 50.0, 70.0]);
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

    #[test]
    fn counts_local_color_features_using_all_three_channel_thresholds() {
        let mut image = RgbaImage::new(4, 2);
        image.put_pixel(1, 1, Rgba([101, 151, 201, 0]));
        image.put_pixel(2, 1, Rgba([100, 255, 255, 255]));
        image.put_pixel(3, 1, Rgba([255, 255, 255, 255]));
        assert_eq!(
            count_rgb_above(
                &ImageRegion::new(&image, Region2D::from_ltwh(0, 1, 3, 1)).unwrap(),
                [100, 150, 200]
            ),
            1
        );
        assert_eq!(
            count_rgb_above(
                &ImageRegion::new(&image, Region2D::from_ltwh(0, 0, 3, 1)).unwrap(),
                [100, 150, 200]
            ),
            0
        );
    }
}
