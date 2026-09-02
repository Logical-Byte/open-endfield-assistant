//! 任务系统模块。
//!
//! `Task` trait 是脚本的扩展点：每个自动化脚本实现一个 Task。
//! [`run_task`] 提供通用启动流程：满足任务的前置场景 → 执行任务。
//!
//! 任务运行中的导航一律委托 [`crate::scene::SceneManager`]，Task 只写业务节奏。

pub mod archive_scan;

use anyhow::Result;

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

    /// 任务执行所需的前置场景。
    ///
    /// [`run_task`] 会在调用 [`Task::run`] 前导航到该场景。
    fn precondition_scene(&self) -> SceneId;

    /// 执行任务主逻辑。
    ///
    /// 运行过程中的临时导航（如进入 / 返回某个界面）委托 `scenes` 完成。
    fn run(&self, session: &mut Session, scenes: &SceneManager) -> Result<()>;
}

/// 运行任务：满足任务的前置场景 → 执行任务。
pub fn run_task(task: &dyn Task, session: &mut Session, scenes: &SceneManager) -> Result<()> {
    tracing::info!("========== 开始执行任务: {} ==========", task.name());

    // 0. 任务开始前先把鼠标移到窗口中心，避免鼠标恰好 hover 在按钮上，
    //    按钮 hover 样式变化干扰首次场景识别 / 导航。
    session.move_mouse_to_safe_position()?;

    // 1. 满足任务的前置场景
    scenes.ensure_scene(task.precondition_scene(), session)?;

    // 2. 执行任务
    task.run(session, scenes)?;

    tracing::info!("========== 任务 {} 执行完毕 ==========", task.name());
    Ok(())
}
