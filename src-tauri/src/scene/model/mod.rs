//! Core scene vocabulary and contracts.

mod scene_id;
mod scene_trait;

pub use crate::automation::{AutomateAction, Key, Point720p, TemplateTarget};
pub use scene_id::{SceneId, 档案库SubSceneId};
pub use scene_trait::{Scene, SceneTransition, execute_transition};
