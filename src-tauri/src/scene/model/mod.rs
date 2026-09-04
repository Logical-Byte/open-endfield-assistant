//! Core scene vocabulary and contracts.

mod scene_action;
mod scene_id;
mod scene_trait;

pub use scene_action::SceneAction;
pub(crate) use scene_action::TAB_ROIS;
pub use scene_id::{SceneId, 档案库SubSceneId};
pub use scene_trait::{Scene, SceneTransition};
