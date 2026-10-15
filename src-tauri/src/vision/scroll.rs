//! 用重叠图像测量垂直滚动位移，与具体页面布局和鼠标操作无关。

use image::RgbaImage;

use crate::utils::region::Region2D;

/// 在给定区域内寻找向上滚动的像素距离。
///
/// `region` 应避开视口边缘动画，且位于两张截图内。`max_shift` 必须小于区域高度，
/// 保证候选位移仍有重叠内容。没有可信的唯一位移时返回 `None`。
pub(crate) fn measure_vertical_shift(
    before: &RgbaImage,
    after: &RgbaImage,
    region: Region2D<u32>,
    max_shift: u32,
) -> Option<u32> {
    let mut candidates: Vec<(u32, f64)> = (0..=max_shift)
        .map(|offset| (offset, difference(before, after, region, offset)))
        .collect();
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
    let &(offset, error) = candidates.first()?;
    let alternative = candidates
        .iter()
        .find(|(other, _)| offset.abs_diff(*other) > 4)?;
    // 平坦区域或重复图案存在多个合理位移，此时不能靠猜测推进扫描游标。
    (error < 9.0 && alternative.1 - error > 0.35).then_some(offset)
}

/// 容忍轻微渲染噪声，判断指定内容区域是否仍停留在原处。
pub(crate) fn unchanged(before: &RgbaImage, after: &RgbaImage, region: Region2D<u32>) -> bool {
    difference(before, after, region, 0) < 1.0
}

fn difference(before: &RgbaImage, after: &RgbaImage, region: Region2D<u32>, offset: u32) -> f64 {
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in (region.y0()..region.y1() - offset).step_by(5) {
        for x in (region.x0()..region.x1()).step_by(7) {
            for channel in 0..3 {
                total += u64::from(
                    before.get_pixel(x, y + offset)[channel]
                        .abs_diff(after.get_pixel(x, y)[channel])
                        .min(60),
                );
                count += 1;
            }
        }
    }
    total as f64 / count as f64
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use super::*;

    fn frame(offset: u32) -> RgbaImage {
        RgbaImage::from_fn(640, 600, |x, y| {
            let seed = x.wrapping_mul(1664525) ^ (y + offset).wrapping_mul(1013904223);
            Rgba([(seed >> 8) as u8, (seed >> 16) as u8, seed as u8, 255])
        })
    }

    #[test]
    fn measures_full_and_partial_scrolls_inside_the_requested_region() {
        let region = Region2D::from_ltrb(10, 20, 620, 580);
        let first = frame(0);
        assert_eq!(
            measure_vertical_shift(&first, &frame(311), region, 375),
            Some(311)
        );
        assert_eq!(
            measure_vertical_shift(&first, &frame(73), region, 375),
            Some(73)
        );
        assert_eq!(measure_vertical_shift(&first, &first, region, 375), Some(0));
        assert!(unchanged(&first, &first, region));
        assert!(!unchanged(&first, &frame(73), region));
    }

    #[test]
    fn does_not_guess_when_repeating_content_hides_the_scroll_distance() {
        let flat = RgbaImage::from_pixel(640, 600, Rgba([55, 55, 55, 255]));
        assert_eq!(
            measure_vertical_shift(&flat, &flat, Region2D::from_ltrb(10, 20, 620, 580), 375),
            None
        );
    }
}
