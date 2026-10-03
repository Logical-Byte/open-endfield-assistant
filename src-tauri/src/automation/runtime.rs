//! 自动化任务的生命周期运行时。
//!
//! 提供全局互斥的任务启动、停止和状态通知。具体任务通过 [`Worker`] adapter 接入。

use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::automation::{
    Event, EventSink, RuntimeEventSink, StopToken, TaskKind, new_stop_token, request_stop,
    stats::counts::CaptureSummary,
};

/// 当前自动化任务的生命周期状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
    Idle {
        /// 最近一次运行的结束信息；初始状态为空，成功启动新任务后移除。
        #[serde(rename = "lastRun")]
        last_run: Option<LastRun>,
    },
    Running {
        #[serde(rename = "runId")]
        run_id: u32,
        #[serde(rename = "taskKind")]
        task_kind: TaskKind,
    },
    Stopping {
        #[serde(rename = "runId")]
        run_id: u32,
        #[serde(rename = "taskKind")]
        task_kind: TaskKind,
    },
}

impl Status {
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle { .. })
    }
}

/// 自动化运行的终态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum RunOutcome {
    Completed,
    Stopped,
    Failed { error: String },
}

/// 空闲状态中保留的最近一次运行结束信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    run_id: u32,
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

/// 一次自动化运行的执行内容。`Runtime` 注入 `StopToken` 和 `EventSink`，每个 `Worker` 只能执行一次。
pub(crate) trait Worker: Send + 'static {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit;
}

/// 全局唯一自动化运行的生命周期状态。
pub(crate) struct Runtime {
    /// 运行状态与当前运行的停止令牌由同一把锁保护，避免停止请求落到错误的运行上。
    state: Mutex<RuntimeState>,
    events: Arc<dyn RuntimeEventSink>,
}

/// 每次运行独立创建。即使事件延迟送达，也始终保留产生它的运行标识。
struct RunEventSink {
    run_id: u32,
    events: Arc<dyn RuntimeEventSink>,
}

impl EventSink for RunEventSink {
    fn publish(&self, event: Event) {
        self.events.publish_run_event(self.run_id, event);
    }
}

struct RuntimeState {
    status: Status,
    next_run_id: u32,
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
    pub(crate) fn new(events: Arc<dyn RuntimeEventSink>) -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                status: Status::Idle { last_run: None },
                next_run_id: 1,
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
        let Some((run_id, stop)) = self.claim_start(task_kind) else {
            warn!("自动化任务正在运行中，忽略重复的启动请求");
            return;
        };
        info!("收到启动自动化任务请求");
        let worker = worker_factory();
        self.emit_status();

        let runtime = Arc::clone(self);
        let events = Arc::new(RunEventSink {
            run_id,
            events: Arc::clone(&self.events),
        });
        if let Err(error) = thread::Builder::new()
            .name("oea-automation".to_string())
            .spawn(move || {
                let result = worker.run(stop, events);
                runtime.handle_worker_exit(run_id, task_kind, result);
            })
        {
            self.handle_worker_exit(
                run_id,
                task_kind,
                WorkerExit::without_capture(FinishReason::Failed(format!(
                    "启动自动化任务线程失败: {error}"
                ))),
            );
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
            Status::Running { run_id, task_kind } => {
                let status = Status::Stopping { run_id, task_kind };
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

    fn claim_start(&self, task_kind: TaskKind) -> Option<(u32, StopToken)> {
        let mut state = self.state.lock().unwrap();
        if state.status.is_active() {
            return None;
        }

        let stop = new_stop_token();
        let run_id = state.next_run_id;
        state.next_run_id += 1;
        state.status = Status::Running { run_id, task_kind };
        state.stop = Some(Arc::clone(&stop));
        Some((run_id, stop))
    }

    /// 处理 worker 退出：记录统计，将结束信息写入空闲状态，再推送完整状态。
    fn handle_worker_exit(&self, run_id: u32, task_kind: TaskKind, result: WorkerExit) {
        let WorkerExit { reason, capture } = result;
        if let Some(summary) = capture {
            let calls = summary.calls;
            info!(
                "自动化任务统计：耗时 {:.1} 秒，截图 {} 次，点击 {} 次，拖动 {} 次，按键 {} 次，\
                 显式鼠标归位 {} 次，模板匹配 {} 次，OCR {} 次，等待 {} 次",
                summary.elapsed.as_secs_f64(),
                calls.screenshot,
                calls.click,
                calls.drag,
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

        self.finish_run_and_emit(LastRun {
            run_id,
            task_kind,
            outcome,
        });
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
        self.events.publish_status(status);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use crate::automation::{
        Event, EventSink, RuntimeEventSink, TaskKind,
        archive_scan::ScannedItem,
        events::testing::{RecordedEvent, RecordingEventSink},
        runtime::{FinishReason, LastRun, RunOutcome, Status, Worker, WorkerExit},
    };

    use super::Runtime;

    struct CompletingWorker(mpsc::Sender<Arc<dyn EventSink>>);

    impl Worker for CompletingWorker {
        fn run(
            self: Box<Self>,
            _stop: crate::automation::StopToken,
            events: Arc<dyn EventSink>,
        ) -> WorkerExit {
            self.0.send(events).unwrap();
            WorkerExit::without_capture(FinishReason::Completed)
        }
    }

    #[test]
    fn keeps_worker_events_bound_to_their_run_after_another_run_starts() {
        let events = Arc::new(RecordingEventSink::default());
        let event_sink: Arc<dyn RuntimeEventSink> = Arc::<RecordingEventSink>::clone(&events);
        let runtime = Arc::new(Runtime::new(event_sink));
        let (sender, receiver) = mpsc::channel();

        runtime.start(TaskKind::ArchiveScan, || {
            Box::new(CompletingWorker(sender.clone()))
        });
        let first_run = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(events.wait_for_events(2).len(), 2);
        runtime.start(TaskKind::ArchiveScan, || Box::new(CompletingWorker(sender)));
        let second_run = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(events.wait_for_events(4).len(), 4);

        let item = Event::ArchiveItemScanned(ScannedItem {
            status: "failed".into(),
            found_in_category: "document".into(),
            found_in_sub_category: "paper".into(),
            image: String::new(),
            ocr_result: String::new(),
            corrected_title: None,
            corrected_match_item_ids: vec![],
        });
        first_run.publish(item.clone());
        second_run.publish(item.clone());

        assert_eq!(
            events.wait_for_events(6),
            vec![
                RecordedEvent::Status(Status::Running {
                    run_id: 1,
                    task_kind: TaskKind::ArchiveScan,
                }),
                RecordedEvent::Status(Status::Idle {
                    last_run: Some(LastRun {
                        run_id: 1,
                        task_kind: TaskKind::ArchiveScan,
                        outcome: RunOutcome::Completed,
                    }),
                }),
                RecordedEvent::Status(Status::Running {
                    run_id: 2,
                    task_kind: TaskKind::ArchiveScan,
                }),
                RecordedEvent::Status(Status::Idle {
                    last_run: Some(LastRun {
                        run_id: 2,
                        task_kind: TaskKind::ArchiveScan,
                        outcome: RunOutcome::Completed,
                    }),
                }),
                RecordedEvent::Run {
                    run_id: 1,
                    event: item.clone()
                },
                RecordedEvent::Run {
                    run_id: 2,
                    event: item
                },
            ]
        );
    }
}
