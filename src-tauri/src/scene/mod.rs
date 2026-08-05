//! 场景系统模块。
//!
//! 借鉴 MaaFramework 的 "识别 → 动作 → next" 节点模型：
//! - **框架**：[`Scene`] trait（自我识别 + 跳转声明）、[`SceneManager`]（注册表 + BFS 导航）、
//!   [`SceneAction`]（跳转动作）、[`SceneId`]（界面唯一标识）；
//! - **本游戏场景实现**：`overworld`（大世界）、`terminal`（协议终端）、`archive`（档案库）。

pub mod archive;
pub mod overworld;
pub mod scene_action;
pub mod scene_id;
pub mod scene_manager;
pub mod scene_trait;
pub mod terminal;

pub use scene_action::SceneAction;
pub use scene_id::{SceneId, 档案库SubSceneId};
pub use scene_manager::SceneManager;
pub use scene_trait::{Scene, SceneTransition};

use std::sync::LazyLock;

use anyhow::Result;

use crate::session::Session;

/// 模板匹配默认阈值（720p 基准）。
pub(crate) const DEFAULT_THRESHOLD: f32 = 0.75;

/// 未知界面（兜底）：所有场景都无法识别时使用；不允许从此场景导航。
pub struct Scene未知;

impl Scene for Scene未知 {
    fn id(&self) -> SceneId {
        SceneId::未知
    }

    fn name(&self) -> &'static str {
        "未知界面"
    }

    fn try_recognize(&self, _session: &mut Session) -> Result<Option<SceneId>> {
        // 未知场景总是返回自身（作为兜底）
        Ok(Some(SceneId::未知))
    }

    fn transitions(&self) -> &[SceneTransition] {
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(Vec::new);
        &T
    }
}

/// 构建并注册本游戏所有场景的 SceneManager。
///
/// 注册顺序即识别优先级（具体 → 笼统）：
/// 档案详情页面 → 档案库子界面 → 档案库主界面 → 协议终端 → 大世界 → 未知（兜底）。
pub fn create_scene_manager() -> SceneManager {
    let mut sm = SceneManager::new();

    sm.register(Box::new(archive::Scene档案详情页面)); // 1. 最具体
    sm.register(Box::new(archive::Scene档案库子界面)); // 2. 子界面
    sm.register(Box::new(archive::Scene档案库主界面)); // 3. 主界面
    sm.register(Box::new(terminal::Scene协议终端)); // 4. 协议终端
    sm.register(Box::new(overworld::Scene大世界)); // 5. 大世界
    sm.register(Box::new(Scene未知)); // 6. 兜底

    sm.build_navigation_graph();
    sm
}
