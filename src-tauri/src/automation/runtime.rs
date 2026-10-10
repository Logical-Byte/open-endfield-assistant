//! 自动化任务的生命周期运行时。
//!
//! 提供全局互斥的任务启动、停止和状态通知。具体任务通过 [`Worker`] adapter 接入。

use crate::automation;
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;
use tracing::{error, info, warn};
use ts_rs::TS;

use crate::automation::{
    Event, EventSink, StopToken, TaskKind, new_stop_token, request_stop,
    stats::counts::CaptureSummary,
};

/// 当前自动化任务的生命周期状态。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export, export_to = "automation/")]
pub enum Status {
    Idle {
        /// 最近一次运行的结束信息；初始状态为空，成功启动新任务后移除。
        #[serde(rename = "lastRun")]
        last_run: Option<LastRun>,
    },
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
        !matches!(self, Self::Idle { .. })
    }
}

/// 自动化运行的终态，由 `status` 区分：
/// - `completed`：工作流执行完毕。
/// - `failed`：运行未完整完成，`error` 携带停止或失败的结构化原因。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
#[ts(export, export_to = "automation/")]
pub enum RunOutcome {
    Completed,
    Failed {
        /// 生命周期或任务执行失败原因，前端按当前语言展示。
        error: automation::Error,
    },
}

/// 空闲状态中保留的最近一次运行结束信息。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "automation/")]
pub struct LastRun {
    task_kind: TaskKind,
    outcome: RunOutcome,
}

/// [`Worker`] 交给 [`Runtime`] 的完整退出信息，不包含任务的领域结果。
pub(crate) struct WorkerExit {
    pub(crate) result: Result<(), automation::WorkerError>,
    pub(crate) capture: Option<CaptureSummary>,
}

/// 一次自动化运行的执行内容。`Runtime` 注入 `StopToken` 和 `EventSink`，每个 `Worker` 只能执行一次。
pub(crate) trait Worker: Send + 'static {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit;
}

/// 全局唯一自动化运行的生命周期状态。
pub(crate) struct Runtime {
    /// 运行状态与当前运行的停止令牌由同一把锁保护，避免停止请求落到错误的运行上。
    state: Mutex<RuntimeState>,
    events: Arc<dyn EventSink>,
}

struct RuntimeState {
    status: Status,
    /// 当前运行的停止令牌。每次成功 `claim` 都创建一个新的令牌。
    stop: Option<StopToken>,
}

impl RuntimeState {
    fn finish(&mut self, last_run: LastRun) {
        self.status = Status::Idle {
            last_run: Some(last_run),
        };
        self.stop = None;
    }
}

impl Runtime {
    /// 接受具体的事件 adapter，在内部擦除类型，调用方可保留其生命周期句柄。
    pub(crate) fn new(events: Arc<impl EventSink>) -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                status: Status::Idle { last_run: None },
                stop: None,
            }),
            events,
        }
    }

    /// 占用全局运行状态并创建本次令牌，再在后台线程执行任务。
    pub(crate) fn start(
        self: &Arc<Self>,
        task_kind: TaskKind,
        worker_factory: impl FnOnce() -> Box<dyn Worker>,
    ) {
        let Some(stop) = self.claim_start(task_kind) else {
            warn!("自动化任务正在运行中，忽略重复的启动请求");
            return;
        };
        info!("收到启动自动化任务请求");
        let worker = worker_factory();
        self.emit_status();

        let runtime = Arc::clone(self);
        let events = Arc::clone(&self.events);
        if let Err(error) = thread::Builder::new()
            .name("oea-automation".to_string())
            .spawn(move || {
                let result = worker.run(stop, events);
                runtime.handle_worker_exit(task_kind, result);
            })
        {
            error!(error = ?error, "启动自动化任务线程失败");
            self.finish_run_and_emit(LastRun {
                task_kind,
                outcome: RunOutcome::Failed {
                    error: automation::RuntimeError::ThreadStartFailed.into(),
                },
            });
        }
    }

    /// 请求停止当前自动化任务。
    pub(crate) fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        let Some(stop) = state.stop.as_ref().map(Arc::clone) else {
            warn!("自动化任务未在运行，忽略停止请求");
            return;
        };
        let status = match state.status {
            Status::Running { task_kind } => {
                let status = Status::Stopping { task_kind };
                state.status = status.clone();
                Some(status)
            }
            Status::Idle { .. } | Status::Stopping { .. } => None,
        };
        request_stop(&stop);
        if let Some(status) = status {
            self.emit_status_value(status);
        }
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
        self.state.lock().unwrap().status.clone()
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

    /// 处理 worker 退出：记录统计，将结束信息写入空闲状态，再推送完整状态。
    fn handle_worker_exit(&self, task_kind: TaskKind, exit: WorkerExit) {
        let WorkerExit { result, capture } = exit;
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

        let outcome = match result {
            Ok(()) => {
                info!("========== 自动化任务执行完毕 ==========");
                RunOutcome::Completed
            }
            Err(error) => {
                if error.is_stopped_by_user() {
                    info!("自动化任务已被用户停止");
                } else {
                    error!("自动化任务失败：{error}");
                }
                RunOutcome::Failed {
                    error: error.into(),
                }
            }
        };

        self.finish_run_and_emit(LastRun { task_kind, outcome });
    }

    fn finish_run_and_emit(&self, last_run: LastRun) {
        let mut state = self.state.lock().unwrap();
        state.finish(last_run);

        // 推送结束状态前继续占用 `claim` 锁，保证它先于下一次运行的状态发出。
        self.emit_status_value(state.status.clone());
    }

    fn emit_status(&self) {
        self.emit_status_value(self.status());
    }

    fn emit_status_value(&self, status: Status) {
        self.events.publish(Event::StatusChanged(status));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::automation;
    use crate::automation::{
        EventSink, StopToken, TaskKind,
        events::testing::RecordingEventSink,
        runtime::{Worker, WorkerExit},
    };

    use super::Runtime;

    struct FinishingWorker(Result<(), automation::WorkerError>);

    impl Worker for FinishingWorker {
        fn run(self: Box<Self>, _stop: StopToken, _events: Arc<dyn EventSink>) -> WorkerExit {
            WorkerExit {
                result: self.0,
                capture: None,
            }
        }
    }

    #[test]
    fn publishes_lifecycle_events_without_tauri() {
        for (result, outcome) in [
            (Ok(()), serde_json::json!({"status": "completed"})),
            (
                Err(automation::capabilities::Error::StoppedByUser.into()),
                serde_json::json!({"status": "failed", "error": {"scope": "worker", "kind": "capability", "reason": {"kind": "stoppedByUser"}}}),
            ),
            (
                Err(automation::capabilities::Error::ExecutionFailed.into()),
                serde_json::json!({"status": "failed", "error": {"scope": "worker", "kind": "capability", "reason": {"kind": "executionFailed"}}}),
            ),
        ] {
            let events = Arc::new(RecordingEventSink::default());
            let runtime = Arc::new(Runtime::new(Arc::clone(&events)));

            runtime.start(TaskKind::ArchiveScan, || Box::new(FinishingWorker(result)));

            let statuses: Vec<_> = events
                .wait_for_events(2)
                .into_iter()
                .map(|event| {
                    let automation::Event::StatusChanged(status) = event else {
                        panic!("生命周期只应发布状态事件");
                    };
                    serde_json::to_value(status).unwrap()
                })
                .collect();
            assert_eq!(
                statuses,
                vec![
                    serde_json::json!({"state": "running", "taskKind": "archiveScan"}),
                    serde_json::json!({"state": "idle", "lastRun": {"taskKind": "archiveScan", "outcome": outcome}}),
                ]
            );
            assert_eq!(serde_json::to_value(runtime.status()).unwrap(), statuses[1]);
        }
    }
}
