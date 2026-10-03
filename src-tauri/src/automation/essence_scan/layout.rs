//! 1280×720 客户区的固定像素位置，依据 EER 的游戏布局标定。

use crate::{
    automation::Point720p,
    utils::{point::Point2D, region::Region2D},
};

pub(super) const COLUMNS: [u32; 9] = [85, 189, 293, 396, 500, 604, 708, 812, 916];
pub(super) const FIRST_ROW_Y: f64 = 130.666_667;
pub(super) const ROW_PITCH: f64 = 103.833_333;
/// 避开视口渐隐与卡片边框动画，用于观察内容是否移动。
pub(super) const SCROLL_CONTENT: Region2D<u32> = Region2D::from_ltrb(55, 97, 939, 620);
pub(super) const SCROLL_TOP: Point2D<u32> = Point2D { x: 969, y: 87 };
pub(super) const SCROLL_BOTTOM: Point2D<u32> = Point2D { x: 969, y: 633 };
pub(super) const DRAG_START: Point720p = Point720p { x: 500, y: 580 };
pub(super) const DRAG_END: Point720p = Point720p { x: 500, y: 269 };

/// 第 `row` 行的绝对背包位置换算到当前滚动视口，相同属性的基质仍各扫一次。
pub(super) fn row_y(row: u32, offset: u32) -> i32 {
    (FIRST_ROW_Y + f64::from(row) * ROW_PITCH - f64::from(offset)).round() as i32
}
