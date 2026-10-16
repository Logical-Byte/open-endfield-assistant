//! 通用图片识别与区域统计。
//!
//! 调用方提供区域、模板和阈值，并将观测结果解释为领域状态。
//! 具体界面的布局、默认识别参数和领域类型归调用方所有。

mod image_region;
pub(crate) use image_region::ImageRegion;

pub(crate) mod ocr;
pub(crate) mod statistics;
pub(crate) mod template_matching;
