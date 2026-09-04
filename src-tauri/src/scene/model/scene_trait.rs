//! 场景 trait 与跳转描述。

use anyhow::Result;

use crate::{
    automation::{AutomateExecutor, AutomateResult},
    session::RecognitionContext,
};

use super::{AutomateAction, SceneId};

/// 场景 trait：每个游戏界面实现此 trait。
///
/// 每个场景负责：
/// 1. 识别自身（`try_recognize`）
/// 2. 定义可跳转的目标场景（`transitions`）
///
/// `Send + Sync`：场景实现均为零大小结构体，自动满足；
/// 同时允许 `Arc<SceneManager>` 跨线程共享（扫描线程 / 命令线程共用）。
pub trait Scene: Send + Sync {
    /// 返回此场景的唯一标识符。
    fn id(&self) -> SceneId;

    /// 返回场景名称（用于日志）。
    fn name(&self) -> &'static str;

    /// 尝试识别当前识别帧是否为此场景。
    ///
    /// # 参数
    /// - `context`: 同一识别帧上的受限识别能力
    ///
    /// # 返回
    /// - `Ok(Some(scene_id))`: 识别成功，返回确切的场景 ID（子界面可能需要返回更具体的 ID）
    /// - `Ok(None)`: 不是此场景
    fn try_recognize(&self, context: &mut RecognitionContext<'_>) -> Result<Option<SceneId>>;

    /// 返回从此场景可以跳转到的目标场景及跳转方式。
    ///
    /// 注意：这是所有可能的跳转，实际执行时会按顺序尝试，第一个成功即停止。
    fn transitions(&self) -> &[SceneTransition];
}

/// 场景跳转描述：从当前场景到目标场景以及执行方式。
pub struct SceneTransition {
    /// 目标场景 ID
    pub target: SceneId,
    /// 跳转动作
    pub action: AutomateAction,
}

pub fn execute_transition(
    scene: &dyn Scene,
    target: SceneId,
    executor: &mut impl AutomateExecutor,
) -> AutomateResult<()> {
    let transition = scene
        .transitions()
        .iter()
        .find(|transition| transition.target == target)
        .ok_or_else(|| anyhow::anyhow!("没有从 {:?} 到 {:?} 的跳转", scene.id(), target))?;
    executor.execute(&transition.action)
}
