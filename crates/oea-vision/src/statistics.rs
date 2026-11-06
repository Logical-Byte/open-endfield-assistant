//! 在已校验的图片区域内统计亮度和颜色，不解释具体界面的状态。

use image::{GenericImageView, Pixel};

use super::ImageRegion;

/// 返回图片指定区域的平均亮度，范围为 `0..=255`。
///
/// 使用 `image` 的 RGB 转灰度权重，忽略 alpha。
pub fn mean_luma<I>(region: &ImageRegion<'_, I>) -> f32
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
