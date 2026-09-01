//! 场景系统模块。
//!
//! - **模型**：[`Scene`] trait（自我识别 + 跳转声明）、[`SceneAction`]（跳转动作）、
//!   [`SceneId`]（界面唯一标识）；
//! - **导航**：[`SceneManager`]（检测、规划与执行）；
//! - **本游戏场景实现**：`scenes`（大世界、协议终端、档案库）。

mod model;
mod route_executor;
mod route_planner;
mod scene_detector;
pub mod scene_manager;
mod scenes;

pub use model::{Scene, SceneAction, SceneId, SceneTransition, 档案库SubSceneId};
pub use scene_manager::SceneManager;
pub use scenes::{Scene未知, archive, overworld, terminal};

/// 模板匹配默认阈值（720p 基准）。
pub(crate) const DEFAULT_THRESHOLD: f32 = 0.75;
