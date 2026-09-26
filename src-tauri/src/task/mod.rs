//! 任务系统模块。
//!
//! `Task` trait 是脚本的扩展点：每个自动化脚本实现一个 Task。
//! [`run_task`] 提供通用启动流程：满足任务的前置具体 UI 状态 → 执行任务。
//!
//! 任务运行中的导航一律委托 [`crate::navigation::Navigator`]，Task 只写业务节奏。

pub mod archive_scan;

use anyhow::Result;

use crate::{
    automation::{Clock, Input, Ocr, ScreenCapture, TemplateMatching},
    navigation::{Navigator, UiState},
};

/// 任务 trait：一个完整的自动化脚本。
///
/// 任务对象在扫描线程内本地构造使用（无需 Send/Sync 约束）。
pub(crate) trait Task {
    /// 任务名称（用于日志）。
    fn name(&self) -> &str;

    /// 任务执行所需的前置具体 UI 状态。
    ///
    /// [`run_task`] 会在调用 [`Task::run`] 前导航到该具体 UI 状态。
    fn precondition_state(&self) -> UiState;

    /// 执行任务主逻辑。
    ///
    /// 运行过程中的临时导航（如进入 / 返回某个界面）委托 `navigator` 完成；具体窗口
    /// 实现隐藏在 `cx` 的细粒度自动化能力之后。
    fn run<C>(&self, cx: &mut C, navigator: &Navigator) -> Result<()>
    where
        C: ScreenCapture + Input + TemplateMatching + Ocr + Clock;
}

/// 运行任务：满足任务的前置具体 UI 状态 → 执行任务。
pub(crate) fn run_task<T, C>(task: &T, cx: &mut C, navigator: &Navigator) -> Result<()>
where
    T: Task,
    C: ScreenCapture + Input + TemplateMatching + Ocr + Clock,
{
    tracing::info!("========== 开始执行任务: {} ==========", task.name());

    // 0. 任务开始前先把鼠标移到窗口中心，避免鼠标恰好 hover 在按钮上，
    //    按钮 hover 样式变化干扰首次 UI 状态识别 / 导航。
    cx.move_mouse_to_safe_position()?;

    // 1. 满足任务的前置具体 UI 状态
    navigator.navigate_to(task.precondition_state(), cx)?;

    // 2. 执行任务
    task.run(cx, navigator)?;

    tracing::info!("========== 任务 {} 执行完毕 ==========", task.name());
    Ok(())
}
