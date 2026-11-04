//! 通用图片识别与区域统计。
//!
//! 调用方提供区域、模板和阈值，并将观测结果解释为领域状态。
//! 具体界面的布局、默认识别参数和领域类型归调用方所有。
//!
//! [`ImageRegion`] 校验借用图片中的非空区域。所有坐标均为输入图片的像素坐标，
//! 不执行分辨率归一化。模板匹配和亮度统计只读取图片并返回计算结果。
//! 模板文件加载与缓存归调用方所有。OCR 引擎负责模型加载，实例的共享与同步归调用方所有。

mod image_region;
mod pixel_rect;
pub use image_region::ImageRegion;
pub use pixel_rect::PixelRect;

pub mod ocr;
pub mod statistics;
pub mod template_matching;
