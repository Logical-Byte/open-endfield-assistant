//! 任务系统模块。
//!
//! 提供 `Task` trait 和 `TaskRunner`，用于定义和执行自动化任务。
//! 每个具体任务（如"扫描档案库"）实现 `Task` trait，由 `TaskRunner` 统一调度。

use anyhow::Result;

use crate::{
    scene::{SceneId, scene_manager::SceneManager},
    session::Session,
};

/// 任务 trait：一个完整的自动化任务。
///
/// 实现此 trait 的类型代表一个具体的游戏自动化任务。
pub trait Task {
    /// 任务名称（用于日志）。
    fn name(&self) -> &str;

    /// 任务支持的入口场景列表。
    ///
    /// 任务可以从这些场景中的任意一个开始执行（TaskRunner 会自动导航到起始场景）。
    fn supported_entry_scenes(&self) -> &[SceneId];

    /// 执行任务主逻辑。
    ///
    /// # 参数
    /// - `session`: 会话上下文
    /// - `scene_manager`: 场景管理器（用于场景检测和导航）
    fn run(&self, session: &mut Session, scene_manager: &SceneManager) -> Result<()>;
}

/// 任务运行器。
///
/// 负责管理场景、检测当前界面、导航到任务入口、执行任务。
pub struct TaskRunner {
    /// 场景管理器
    pub scene_manager: SceneManager,
}

impl TaskRunner {
    /// 创建新的 TaskRunner。
    ///
    /// # 参数
    /// - `scene_manager`: 已注册所有场景并构建导航图的 SceneManager
    pub fn new(scene_manager: SceneManager) -> Self {
        Self { scene_manager }
    }

    /// 运行任务。
    ///
    /// 自动检测当前场景，如果不在任务支持的入口场景中则报错，
    /// 否则导航到第一个支持的入口场景并执行任务。
    pub fn run_task(&mut self, task: &dyn Task, session: &mut Session) -> Result<()> {
        tracing::info!("========== 开始执行任务: {} ==========", task.name());

        // 1. 检测当前场景
        let current = self.scene_manager.detect_current_scene(session)?;

        // 2. 检查是否在受支持的入口场景中
        let entries = task.supported_entry_scenes();
        if !entries.contains(&current) {
            // 尝试找到最近的受支持场景并导航过去
            // 先尝试导航到第一个入口场景
            let target = entries.first().copied().unwrap_or(SceneId::未知);
            if target == SceneId::未知 {
                anyhow::bail!("任务 {} 无有效入口场景", task.name());
            }
            tracing::info!(
                "当前场景 {:?} 不在任务入口列表中，导航到 {:?}",
                current,
                target
            );
            self.scene_manager.navigate_to(target, session)?;
        }

        // 3. 执行任务
        task.run(session, &self.scene_manager)?;

        tracing::info!("========== 任务 {} 执行完毕 ==========", task.name());
        Ok(())
    }
}
