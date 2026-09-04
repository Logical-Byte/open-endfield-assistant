//! 档案库扫描任务共享的坐标与阈值常量。
//!
//! 所有坐标均基于 1280×720 基准分辨率，识别时截图会先缩放到此分辨率。

use crate::utils::region::{Region2D, ltrb, ltwh};

/// 模板匹配默认阈值
pub const THRESHOLD: f32 = 0.75;

/// 档案标题 OCR 识别区域（720p 基准 ltwh）
pub const OCR_ROI: Region2D<u32> = ltwh!(350, 58, 578, 42);

/// "下一篇" 按钮搜索区域（720p 基准 ltrb）
pub const NEXT_BUTTON_ROI: Region2D<u32> = ltrb!(762, 654, 925, 711);

/// "档案详情右箭头" 搜索区域（720p 基准 ltrb）
pub const ARROW_RIGHT_ROI: Region2D<u32> = ltrb!(1206, 313, 1276, 423);

/// "档案详情关闭" 按钮搜索区域（720p 基准 ltrb）
pub const CLOSE_BUTTON_ROI: Region2D<u32> = ltrb!(1180, 0, 1280, 100);
