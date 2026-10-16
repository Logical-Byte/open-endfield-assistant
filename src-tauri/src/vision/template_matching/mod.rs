//! 模板匹配模块（基础设施，通用库）。
//!
//! 基于归一化互相关（`ccoeff`）在截图 ROI 内搜索模板图片，返回匹配分数与区域。
//! - [`LazyTemplateLoader`]：按模板名懒加载并缓存图片（首次使用读盘）；
//! - [`find`]：通过模板提供者取得命名模板并在图片区域内搜索；
//! - [`pure::match_template`]：直接使用已加载的模板图片进行匹配；

mod ccoeff;
mod matching;
mod template_source;

pub(crate) use matching::{find, pure};
pub(crate) use template_source::LazyTemplateLoader;
