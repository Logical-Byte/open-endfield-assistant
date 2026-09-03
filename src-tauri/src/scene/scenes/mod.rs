//! Concrete game scene recognizers and transition definitions.

pub mod archive;
pub mod overworld;
pub mod terminal;
mod unknown;

/// 模板匹配阈值（720p 基准）。
const TEMPLATE_MATCH_THRESHOLD: f32 = 0.75;

pub use unknown::Scene未知;
