//! 固定 16:9、1280×720 布局。
//!
//! 来源：EER commit `066d0e5b` 的 `core/layout/res_1080p.py`。
//! 下方点和 ROI 由该文件同名 1920×1080 常量乘 `2/3`，点四舍五入，
//! ROI 向外取整。模板同步缩放为 720p。尚未经过 Windows 实机校准。
//! 单独注明的拖动起点、步长和视口边界为 OEA 从网格布局推导的策略。

use crate::{automation::Point720p, utils::region::Region2D};

// essence_icon_x_list = np.linspace(128, 1374, 9).astype(int)，再缩放。
pub(super) const COLUMNS: [u32; 9] = [85, 189, 293, 397, 501, 604, 708, 812, 916];
// essence_icon_y_list = np.linspace(196, 819, 5)。保留分数，避免逐行舍入累积误差。
pub(super) const FIRST_ROW_Y: f64 = 196.0 * 2.0 / 3.0;
pub(super) const ROW_PITCH: f64 = (819.0 - 196.0) / 4.0 * 2.0 / 3.0;
pub(super) const TITLE: Region2D<u32> = Region2D::from_ltrb(25, 44, 96, 71);
pub(super) const STATS: [Region2D<u32>; 3] = [
    Region2D::from_ltrb(1005, 238, 1134, 260),
    Region2D::from_ltrb(1005, 277, 1134, 299),
    Region2D::from_ltrb(1005, 312, 1134, 334),
];
pub(super) const LOCK: Region2D<u32> = Region2D::from_ltrb(1216, 180, 1238, 202);
pub(super) const ABANDON: Region2D<u32> = Region2D::from_ltrb(1193, 180, 1216, 202);
pub(super) const RARITY: Region2D<u32> = Region2D::from_ltrb(978, 52, 982, 55);
pub(super) const DETAIL: Region2D<u32> = Region2D::from_ltrb(976, 52, 1256, 355);
// STATS_LEVEL_ICON_POINTS 的 np.linspace(1503,1588,6)、np.linspace(395,507,3)。
pub(super) const LEVEL_X: [u32; 6] = [1002, 1013, 1025, 1036, 1047, 1059];
pub(super) const LEVEL_Y: [u32; 3] = [263, 301, 338];
pub(super) const SCROLL_X: u32 = 969; // SCROLLBAR_*_CHECK_POS.x = 1453。
pub(super) const SCROLL_TOP: u32 = 87; // SCROLLBAR_TOP_CHECK_POS.y = 130。
pub(super) const SCROLL_BOTTOM: u32 = 633; // SCROLLBAR_CHECK_POS.y = 950。
pub(super) const DRAG_START: Point720p = Point720p { x: 500, y: 580 }; // (750,870)。
// EER 原始拖动终点为 (750,50)。OEA 每次只拖三行，保留两行图像重叠以测量真实位移。
pub(super) const DRAG_END: Point720p = Point720p { x: 500, y: 269 };
// OEA 回顶拖动起点：沿用 EER 拖动的 x=750（缩放后500），y=160 位于首行卡片内部。
// 与 DRAG_START 构成向下拖动，不依赖当前选中物品的详情内容。
pub(super) const RESET_START: Point720p = Point720p { x: 500, y: 160 };
// OEA 从 EER 网格包围区域向内缩进，避开边缘渐隐、滚动条和顶部工具栏。
pub(super) const SCROLL_CONTENT: Region2D<u32> = Region2D::from_ltrb(55, 97, 939, 620);
// 点击中心限制在网格内部，MIN_ROW_Y 为 OEA 的保守上沿，仍包含 EER 首行中心130.67。
pub(super) const MIN_ROW_Y: i32 = 125;
// EER dynamic.py：容器底边 1080-120=960，卡片高145。完整末行中心下界 (960-145/2)*2/3≈592。
// 末页可能停在这里，不能只按首页最后一行中心546判断。
pub(super) const MAX_ROW_Y: i32 = 592;

pub(super) fn row_y(row: u32, offset: u32) -> i32 {
    (FIRST_ROW_Y + f64::from(row) * ROW_PITCH - f64::from(offset)).round() as i32
}
