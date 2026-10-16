use image::{RgbImage, imageops};
use imageproc::contrast::{ThresholdType, threshold};

use crate::utils::region::Region2D;

/// 检测单行连续文字（黑底白字或白底黑字）。
///
/// 黑底白字时，阈值类型使用 [`ThresholdType::Binary`]；
/// 白底黑字时，阈值类型使用 [`ThresholdType::BinaryInverted`]。
///
/// 简化版：二值化后扫描全图找出所有白色像素的整体包围框，不做连通域分析和合并。
/// 适用于保证只有单行连续文字的场景。
pub(crate) fn detect_single_line(
    image: &RgbImage,
    threshold_value: u8,
    threshold_type: ThresholdType,
    padding: u32,
) -> Option<Region2D<u32>> {
    let gray = imageops::grayscale(image);
    let binary = threshold(&gray, threshold_value, threshold_type);

    let (w, h) = binary.dimensions();
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = u32::MIN;
    let mut max_y = u32::MIN;
    let mut found = false;

    for y in 0..h {
        for x in 0..w {
            if binary.get_pixel(x, y).0[0] >= 128 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return None;
    }

    let region = Region2D::from_ltrb(min_x, min_y, max_x + 1, max_y + 1);
    Some(apply_padding(region, padding, w, h))
}

/// 给包围框加 padding，并裁剪到图像边界内
fn apply_padding(region: Region2D<u32>, padding: u32, img_w: u32, img_h: u32) -> Region2D<u32> {
    let left = region.x0().saturating_sub(padding);
    let top = region.y0().saturating_sub(padding);
    let right = (region.x1() + padding).min(img_w);
    let bottom = (region.y1() + padding).min(img_h);

    Region2D::from_ltrb(left, top, right, bottom)
}
