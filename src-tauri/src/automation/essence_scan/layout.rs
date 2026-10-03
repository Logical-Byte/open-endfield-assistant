//! 1280×720 客户区的固定像素位置，依据 EER 的游戏布局标定。

use crate::{automation::Point720p, utils::region::Region2D};

pub(super) const COLUMNS: [u32; 9] = [85, 189, 293, 396, 500, 604, 708, 812, 916];
pub(super) const FIRST_ROW_Y: f64 = 130.666_667;
pub(super) const ROW_PITCH: f64 = 103.833_333;
// 网格视口的固定边界。每次推进保留两行重叠，用于测量实际滚动位移。
pub(super) const GRID: Region2D<u32> = Region2D::from_ltrb(40, 85, 954, 632);
pub(super) const SCENE: Region2D<u32> = Region2D::from_ltrb(20, 38, 102, 78);
pub(super) const DETAIL: Region2D<u32> = Region2D::from_ltrb(974, 45, 1260, 360);
pub(super) const STATS: [Region2D<u32>; 3] = [
    Region2D::from_ltrb(1002, 235, 1138, 263),
    Region2D::from_ltrb(1002, 274, 1138, 301),
    Region2D::from_ltrb(1002, 309, 1138, 336),
];
pub(super) const LOCK: Region2D<u32> = Region2D::from_ltrb(1214, 177, 1242, 205);
pub(super) const ABANDON: Region2D<u32> = Region2D::from_ltrb(1190, 177, 1218, 205);
pub(super) const LEVEL_X: [u32; 6] = [1002, 1013, 1025, 1036, 1047, 1059];
pub(super) const LEVEL_Y: [u32; 3] = [263, 301, 338];
pub(super) const SCROLL_TOP: Point720p = Point720p { x: 969, y: 87 };
pub(super) const SCROLL_BOTTOM: Point720p = Point720p { x: 969, y: 633 };
pub(super) const DRAG_START: Point720p = Point720p { x: 500, y: 580 };
pub(super) const DRAG_END: Point720p = Point720p { x: 500, y: 269 };
