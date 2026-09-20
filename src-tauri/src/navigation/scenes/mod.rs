//! Scene vocabulary, recognizers, and transition definitions.

pub mod archive;
pub mod overworld;
mod scene_id;
mod scene_trait;
pub mod terminal;
mod unknown;

/// 模板匹配阈值（720p 基准）。
const TEMPLATE_MATCH_THRESHOLD: f32 = 0.75;

pub use scene_id::{SceneId, 档案库SubSceneId};
pub use scene_trait::Scene;
pub use unknown::Scene未知;
