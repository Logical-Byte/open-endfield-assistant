//! 任务系统模块。
//!
//! `Task` trait 是脚本的扩展点：每个自动化脚本（扫描档案库、刷战令……）实现一个 Task。
//! [`run_task`] 提供通用启动流程：检测当前场景 → 不在入口列表则导航到第一个入口 → 执行任务。
//!
//! 任务运行中的导航一律委托 [`crate::scene::SceneManager`]，Task 只写业务节奏。

use anyhow::{Result, bail};

use crate::{
    scene::{SceneId, scene_manager::SceneManager},
    session::Session,
};

/// 任务被用户停止的错误信号。
///
/// 停止不是"任务出错"：上层用 `downcast_ref::<TaskStopped>()` 区分
/// "被停止"与"出错"，避免把停止当作异常处理。
#[derive(Debug)]
pub struct TaskStopped;

impl std::fmt::Display for TaskStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "任务已被用户停止")
    }
}

impl std::error::Error for TaskStopped {}

/// 任务 trait：一个完整的自动化脚本。
///
/// 任务对象在扫描线程内本地构造使用（无需 Send/Sync 约束）。
pub trait Task {
    /// 任务名称（用于日志）。
    fn name(&self) -> &str;

    /// 任务支持的入口场景列表。
    ///
    /// 任务可以从这些场景中的任意一个开始执行；不在列表中时 [`run_task`]
    /// 会导航到第一个入口场景。
    fn supported_entry_scenes(&self) -> &[SceneId];

    /// 执行任务主逻辑。
    ///
    /// 运行过程中的临时导航（如进入 / 返回某个界面）委托 `scenes` 完成。
    fn run(&self, session: &mut Session, scenes: &SceneManager) -> Result<()>;
}

/// 运行任务：检测当前场景 → 不在入口列表则导航到第一个入口 → 执行任务。
pub fn run_task(task: &dyn Task, session: &mut Session, scenes: &SceneManager) -> Result<()> {
    tracing::info!("========== 开始执行任务: {} ==========", task.name());

    // 1. 检测当前场景
    let current = scenes.detect_current_scene(session)?;

    // 2. 不在支持的入口场景中则导航到第一个入口
    if !task.supported_entry_scenes().contains(&current) {
        let target = task
            .supported_entry_scenes()
            .first()
            .copied()
            .unwrap_or(SceneId::未知);
        if target == SceneId::未知 {
            bail!("任务 {} 无有效入口场景", task.name());
        }
        tracing::info!(
            "当前场景 {:?} 不在任务入口列表中，导航到 {:?}",
            current,
            target
        );
        scenes.navigate_to(target, session)?;
    }

    // 3. 执行任务
    task.run(session, scenes)?;

    tracing::info!("========== 任务 {} 执行完毕 ==========", task.name());
    Ok(())
}
