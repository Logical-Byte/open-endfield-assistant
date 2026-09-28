//! 自动化任务的运行时。
//!
//! [`Runtime`] 管理全局唯一自动化任务的运行状态、停止令牌与工作线程。
//! 真实游戏操作和领域结果上报由调用方提供的 [`Worker`] 执行。

use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

use crate::automation::{
    StopToken, TaskKind, new_stop_token, request_stop, stats::counts::CaptureSummary,
};

/// 当前自动化任务的生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
    Idle,
    Running {
        #[serde(rename = "taskKind")]
        task_kind: TaskKind,
    },
    Stopping {
        #[serde(rename = "taskKind")]
        task_kind: TaskKind,
    },
}

impl Status {
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle)
    }
}

/// 自动化运行的终态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum RunOutcome {
    Completed,
    Stopped,
    Failed { error: String },
}

/// 自动化运行结束时向前端推送的一次性通知。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunFinished {
    task_kind: TaskKind,
    outcome: RunOutcome,
}

/// 工作者结束的原因；失败时保留供运行时记录和展示的错误信息。
pub(crate) enum FinishReason {
    Completed,
    Stopped,
    Failed(String),
}

/// [`Worker`] 交给 [`Runtime`] 的完整退出信息，不包含任务的领域结果。
pub(crate) struct WorkerExit {
    pub(crate) reason: FinishReason,
    pub(crate) capture: Option<CaptureSummary>,
}

impl WorkerExit {
    pub(crate) fn without_capture(reason: FinishReason) -> Self {
        Self {
            reason,
            capture: None,
        }
    }

    pub(crate) fn with_capture(reason: FinishReason, capture: CaptureSummary) -> Self {
        Self {
            reason,
            capture: Some(capture),
        }
    }
}

/// 一次自动化运行的执行内容。按值接收工作者，避免同一个工作者重复执行。
pub(crate) trait Worker: Send + 'static {
    fn run(self: Box<Self>, stop: StopToken) -> WorkerExit;
}

/// 全局唯一自动化运行的生命周期状态。
pub(crate) struct Runtime {
    /// 运行状态与当前运行的停止令牌由同一把锁保护，避免停止请求落到错误的运行上。
    state: Mutex<RuntimeState>,
}

struct RuntimeState {
    status: Status,
    /// 当前运行的停止令牌。每次成功 claim 都创建一个新的令牌。
    stop: Option<StopToken>,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                status: Status::Idle,
                stop: None,
            }),
        }
    }

    /// 占用全局运行状态并创建本次令牌，再在后台线程执行任务。
    pub(crate) fn start(
        self: &Arc<Self>,
        handle: &AppHandle,
        task_kind: TaskKind,
        worker_factory: impl FnOnce() -> Box<dyn Worker>,
    ) {
        let Some(stop) = self.claim_start(task_kind) else {
            warn!("自动化任务正在运行中，忽略重复的启动请求");
            return;
        };
        info!("收到启动自动化任务请求");
        let worker = worker_factory();
        self.emit_status(handle);

        let runtime = Arc::clone(self);
        let handle = handle.clone();
        thread::Builder::new()
            .name("oea-automation".to_string())
            .spawn(move || {
                let result = worker.run(stop);
                runtime.handle_worker_exit(&handle, task_kind, result);
            })
            .expect("启动自动化任务线程失败");
    }

    /// 请求停止当前自动化任务。
    pub(crate) fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        let Some(stop) = state.stop.as_ref().map(Arc::clone) else {
            warn!("自动化任务未在运行，忽略停止请求");
            return;
        };
        if let Status::Running { task_kind } = state.status {
            state.status = Status::Stopping { task_kind };
        }
        request_stop(&stop);
        info!("收到停止请求，正在停止自动化任务...");
    }

    /// 进程退出前请求停止，不改变原有的立即退出策略。
    pub(crate) fn request_stop_for_shutdown(&self) {
        let state = self.state.lock().unwrap();
        if let Some(stop) = state.stop.as_ref() {
            request_stop(stop);
        }
    }

    /// 读取当前自动化状态。
    pub(crate) fn status(&self) -> Status {
        self.state.lock().unwrap().status
    }

    fn claim_start(&self, task_kind: TaskKind) -> Option<StopToken> {
        let mut state = self.state.lock().unwrap();
        if state.status.is_active() {
            return None;
        }

        let stop = new_stop_token();
        state.status = Status::Running { task_kind };
        state.stop = Some(Arc::clone(&stop));
        Some(stop)
    }

    /// 处理 worker 退出：记录统计、释放运行状态，再推送状态与一次性终态。
    fn handle_worker_exit(&self, handle: &AppHandle, task_kind: TaskKind, result: WorkerExit) {
        let WorkerExit { reason, capture } = result;
        if let Some(summary) = capture {
            let calls = summary.calls;
            info!(
                "自动化任务统计：耗时 {:.1} 秒，截图 {} 次，点击 {} 次，按键 {} 次，\
                 显式鼠标归位 {} 次，模板匹配 {} 次，OCR {} 次，等待 {} 次",
                summary.elapsed.as_secs_f64(),
                calls.screenshot,
                calls.click,
                calls.press_key,
                calls.move_mouse_to_safe_position,
                calls.find_template,
                calls.recognize_text,
                calls.sleep,
            );
        }

        let outcome = match reason {
            FinishReason::Completed => {
                info!("========== 自动化任务执行完毕 ==========");
                RunOutcome::Completed
            }
            FinishReason::Stopped => {
                info!("自动化任务已被用户停止");
                RunOutcome::Stopped
            }
            FinishReason::Failed(message) => {
                error!("{message}");
                RunOutcome::Failed { error: message }
            }
        };

        self.finish_run();
        self.emit_status(handle);
        self.emit_run_finished(handle, RunFinished { task_kind, outcome });
    }

    fn finish_run(&self) {
        let mut state = self.state.lock().unwrap();
        state.status = Status::Idle;
        state.stop = None;
    }

    fn emit_status(&self, handle: &AppHandle) {
        if let Err(error) = handle.emit("automation-status", self.status()) {
            error!("向前端推送自动化状态失败: {error}");
        }
    }

    fn emit_run_finished(&self, handle: &AppHandle, event: RunFinished) {
        if let Err(error) = handle.emit("automation-run-finished", event) {
            error!("向前端推送自动化任务终态失败: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::automation::{TaskKind, is_stop_requested};

    use super::{Runtime, Status};

    #[test]
    fn stop_transitions_the_active_task_to_stopping() {
        let runtime = Runtime::new();

        assert!(runtime.claim_start(TaskKind::ArchiveScan).is_some());
        runtime.stop();

        assert_eq!(
            runtime.status(),
            Status::Stopping {
                task_kind: TaskKind::ArchiveScan,
            }
        );
    }

    #[test]
    fn new_runtime_is_idle() {
        assert_eq!(Runtime::new().status(), Status::Idle);
    }

    #[test]
    fn running_task_rejects_a_second_claim_until_it_finishes() {
        let runtime = Runtime::new();

        assert!(runtime.claim_start(TaskKind::ArchiveScan).is_some());
        assert!(runtime.claim_start(TaskKind::ArchiveScan).is_none());

        // 模拟工作线程处理终态后释放运行标志。
        runtime.finish_run();
        assert!(runtime.claim_start(TaskKind::ArchiveScan).is_some());
    }

    #[test]
    fn stop_request_is_ignored_while_idle_and_recorded_while_running() {
        let runtime = Runtime::new();

        runtime.stop();
        assert_eq!(runtime.status(), Status::Idle);

        assert!(runtime.claim_start(TaskKind::ArchiveScan).is_some());
        runtime.stop();
        assert!(is_stop_requested(
            runtime.state.lock().unwrap().stop.as_ref().unwrap()
        ));
    }

    #[test]
    fn each_claim_gets_a_fresh_clear_stop_token() {
        let runtime = Runtime::new();

        let first_stop = runtime.claim_start(TaskKind::ArchiveScan).unwrap();
        runtime.stop();
        assert!(is_stop_requested(&first_stop));

        runtime.finish_run();
        let second_stop = runtime.claim_start(TaskKind::ArchiveScan).unwrap();
        assert!(!is_stop_requested(&second_stop));
        assert!(!Arc::ptr_eq(&first_stop, &second_stop));
    }
}
