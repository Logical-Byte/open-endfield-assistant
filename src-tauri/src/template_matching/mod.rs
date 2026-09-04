//! 模板匹配模块（基础设施，通用库）。
//!
//! 基于归一化互相关（`ccoeff`）在截图 ROI 内搜索模板图片，返回匹配分数与区域。
//! - [`LazyTemplateLoader`]：按模板名懒加载并缓存图片（首次使用读盘）；
//! - [`match_template_in_region`]：单张模板在区域内匹配；
//! - [`MatchResult`]：匹配结果（分数 + 区域）。

mod ccoeff;
mod match_template;
mod template_source;

pub use ccoeff::*;
pub use match_template::*;
pub(crate) use template_source::{LazyTemplateLoader, TemplateSource};
