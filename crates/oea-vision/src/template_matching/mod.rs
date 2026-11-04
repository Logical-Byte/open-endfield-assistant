//! 在已校验的图片区域内搜索已加载的模板，返回原图坐标和匹配分数。
//! 模板加载、缓存和匹配阈值由调用方负责。

mod ccoeff;
mod matching;

pub use matching::{MatchResult, match_template};
