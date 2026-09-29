//! 模板匹配模块（基础设施，通用库）。
//!
//! 基于归一化互相关（`ccoeff`）在截图 ROI 内搜索模板图片，返回匹配分数与区域。
//! - [`LazyTemplateLoader`]：按模板名懒加载并缓存图片（首次使用读盘）；
//! - [`find`]：通过模板提供者取得命名模板并在图片区域内搜索；
//! - [`match_template_in_region`]、[`match_in_region`]：使用已加载模板的纯匹配；
//! - [`MatchResult`]：匹配结果（分数 + 区域）。

mod ccoeff;
mod match_template;
mod template_source;

pub use match_template::{
    MatchResult, find, find_in_region, match_in_region, match_template_in_region,
};
pub(crate) use template_source::LazyTemplateLoader;
pub use template_source::TemplateProvider;
