//! 场景 trait 与跳转描述。

use anyhow::Result;

use crate::session::Session;

use super::{SceneAction, SceneId};

/// 场景 trait：每个游戏界面实现此 trait。
///
/// 借鉴 MaaFramework Pipeline 节点的设计，每个场景负责：
/// 1. 识别自身（`try_recognize`）
/// 2. 定义可跳转的目标场景（`transitions`）
pub trait Scene {
    /// 返回此场景的唯一标识符。
    fn id(&self) -> SceneId;

    /// 返回场景名称（用于日志）。
    fn name(&self) -> &'static str;

    /// 尝试识别当前截图是否为此场景。
    ///
    /// # 参数
    /// - `session`: 会话上下文（从中获取截图等）
    ///
    /// # 返回
    /// - `Ok(Some(scene_id))`: 识别成功，返回确切的场景 ID（子界面可能需要返回更具体的 ID）
    /// - `Ok(None)`: 不是此场景
    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>>;

    /// 返回从此场景可以跳转到的目标场景及跳转方式。
    ///
    /// 注意：这是所有可能的跳转，实际执行时会按顺序尝试，第一个成功即停止。
    fn transitions(&self) -> &[SceneTransition];

    /// 执行跳转动作以到达目标场景。
    ///
    /// 默认实现：从 `transitions()` 中找到去往 `target` 的动作并执行。
    fn execute_transition(&self, target: SceneId, session: &mut Session) -> Result<()> {
        let screenshot = session.screencap_for_recognition()?;
        for transition in self.transitions() {
            if transition.target == target {
                transition.action.execute(session, &screenshot)?;
                return Ok(());
            }
        }
        anyhow::bail!("没有从 {:?} 到 {:?} 的跳转", self.id(), target);
    }
}

/// 场景跳转描述：从当前场景到目标场景以及执行方式。
pub struct SceneTransition {
    /// 目标场景 ID
    pub target: SceneId,
    /// 跳转动作
    pub action: SceneAction,
}
