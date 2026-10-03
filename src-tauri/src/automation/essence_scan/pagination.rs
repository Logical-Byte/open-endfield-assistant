//! 用两帧网格的重叠部分测量实际滚动距离，避免按鼠标拖动距离推算尾页。

use image::RgbaImage;

use super::layout;

pub(super) fn measure_scroll(before: &RgbaImage, after: &RgbaImage) -> Option<u32> {
    let mut candidates: Vec<(u32, f64)> = (0..=375)
        .map(|offset| (offset, difference(before, after, offset)))
        .collect();
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (offset, error) = candidates[0];
    let alternative = candidates
        .iter()
        .find(|(other, _)| offset.abs_diff(*other) > 4)?;
    // 如果两种位移同样合理，不冒险跨过未扫描的行。用户可以保留已有结果并重试。
    (error < 9.0 && alternative.1 - error > 0.35).then_some(offset)
}

fn difference(before: &RgbaImage, after: &RgbaImage, offset: u32) -> f64 {
    let mut total = 0_u64;
    let mut count = 0_u64;
    // 避开视口上下边缘的渐隐与裁切，以及卡片边框的选中动画。
    for y in (layout::GRID.y0() + 12..layout::GRID.y1() - offset - 12).step_by(5) {
        for x in (layout::GRID.x0() + 15..layout::GRID.x1() - 15).step_by(7) {
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

pub(super) fn unchanged(before: &RgbaImage, after: &RgbaImage) -> bool {
    difference(before, after, 0) < 1.0
        && (85..635).all(|y| {
            (966..=972).all(|x| {
                before.get_pixel(x, y).0[..3]
                    .iter()
                    .zip(&after.get_pixel(x, y).0[..3])
                    .all(|(a, b)| a.abs_diff(*b) < 15)
            })
        })
}

/// 第 `row` 行在当前视口中的位置。游标按背包位置推进，相同属性的基质仍各扫一次。
pub(super) fn row_y(row: u32, offset: u32) -> i32 {
    (layout::FIRST_ROW_Y + f64::from(row) * layout::ROW_PITCH - f64::from(offset)).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn frame(offset: u32) -> RgbaImage {
        RgbaImage::from_fn(1280, 720, |x, y| {
            // 非周期纹理模拟不同卡片，不依赖物品属性的唯一性。
            let seed = x.wrapping_mul(1664525) ^ (y + offset).wrapping_mul(1013904223);
            Rgba([(seed >> 8) as u8, (seed >> 16) as u8, seed as u8, 255])
        })
    }

    #[test]
    fn measures_a_full_drag_and_a_partial_last_page() {
        let first = frame(0);
        assert_eq!(measure_scroll(&first, &frame(311)), Some(311));
        assert_eq!(measure_scroll(&first, &frame(73)), Some(73));
        assert_eq!(measure_scroll(&first, &first), Some(0));
        assert_eq!(row_y(5, 311), 339);
        assert_eq!(row_y(8, 622), 339);
    }

    #[test]
    fn does_not_guess_when_identical_rows_hide_the_scroll_distance() {
        let flat = RgbaImage::from_pixel(1280, 720, Rgba([55, 55, 55, 255]));
        assert_eq!(measure_scroll(&flat, &flat), None);
    }
}
