//! 场景系统模块。
//!
//! 借鉴 MaaFramework 的 "识别 → 动作 → next" 节点模型，每个游戏界面实现
//! [`Scene`] trait，负责自我识别和跳转。

pub mod scene_action;
pub mod scene_id;
pub mod scene_manager;
pub mod scene_trait;

pub use scene_action::SceneAction;
pub use scene_id::{SceneId, 档案库SubSceneId};
pub use scene_trait::{Scene, SceneTransition};
