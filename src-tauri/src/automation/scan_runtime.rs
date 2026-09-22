//! 扫描档案库任务的运行时。
//!
//! [`ScanRuntime`] 只拥有一次扫描任务的生命周期状态。工作线程的执行内容
//! 由调用方注入；运行时不会连接游戏或持有物理自动化能力。

use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info, warn};

use crate::automation::{
    StopToken, new_stop_token, request_stop,
    stats::counts::{CapabilityCallCounts, CaptureSummary},
};

/// 推送给前端的应用状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    /// 扫描档案库任务是否正在运行
    pub running: bool,
    /// 扫描档案库任务结束时的失败原因（仅失败时随结束状态推送一次；成功 / 被停止 / 查询状态时为 `None`）
    #[serde(default)]
    pub scan_error: Option<String>,
}

/// 前端可区分的自动化任务。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationTask {
    ArchiveScan,
}

/// 一次自动化运行的终态。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationOutcome {
    Completed,
    Stopped,
    Failed,
}

impl AutomationOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

/// 推送给前端的一次自动化运行结束事件。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunFinished {
    task: AutomationTask,
    outcome: AutomationOutcome,
    capture: Option<CaptureSummaryPayload>,
}

impl AutomationRunFinished {
    fn archive_scan(outcome: AutomationOutcome, capture: Option<CaptureSummary>) -> Self {
        Self {
            task: AutomationTask::ArchiveScan,
            outcome,
            capture: capture.map(CaptureSummaryPayload::from),
        }
    }
}

/// [`CaptureSummary`] 在 Tauri 事件中的稳定序列化格式。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureSummaryPayload {
    elapsed_micros: u64,
    calls: CapabilityCallCountsPayload,
}

impl From<CaptureSummary> for CaptureSummaryPayload {
    fn from(summary: CaptureSummary) -> Self {
        Self {
            elapsed_micros: u64::try_from(summary.elapsed.as_micros()).unwrap_or(u64::MAX),
            calls: summary.calls.into(),
        }
    }
}

/// [`CapabilityCallCounts`] 在 Tauri 事件中的稳定序列化格式。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityCallCountsPayload {
    screenshot: u64,
    click: u64,
    press_key: u64,
    move_mouse_to_safe_position: u64,
    find_template: u64,
    recognize_text: u64,
    sleep: u64,
}

impl From<CapabilityCallCounts> for CapabilityCallCountsPayload {
    fn from(counts: CapabilityCallCounts) -> Self {
        Self {
            screenshot: counts.screenshot,
            click: counts.click,
            press_key: counts.press_key,
            move_mouse_to_safe_position: counts.move_mouse_to_safe_position,
            find_template: counts.find_template,
            recognize_text: counts.recognize_text,
            sleep: counts.sleep,
        }
    }
}

/// 工作者结束的原因；失败时保留供运行时记录和展示的错误信息。
pub(crate) enum FinishReason {
    Completed,
    Interrupted,
    Failed(String),
}

impl FinishReason {
    const fn public_outcome(&self) -> AutomationOutcome {
        match self {
            Self::Completed => AutomationOutcome::Completed,
            Self::Interrupted => AutomationOutcome::Stopped,
            Self::Failed(_) => AutomationOutcome::Failed,
        }
    }
}

/// `ScanWorker` 交给 `ScanRuntime` 的终态数据，与逐条上报的 `ScanResult` 不同。
pub(crate) struct ScanWorkerExit {
    pub(crate) reason: FinishReason,
    pub(crate) capture: Option<CaptureSummary>,
}

impl ScanWorkerExit {
    pub(crate) fn without_capture(reason: FinishReason) -> Self {
        Self {
            reason,
            capture: None,
        }
    }
}

/// 一次扫描工作线程的执行内容。消费工作者，保证每次获准启动只执行一次。
pub(crate) trait ScanWorker: Send + 'static {
    fn run(self, stop: StopToken) -> ScanWorkerExit;
}

/// 扫描档案库任务的生命周期状态。
pub(crate) struct ScanRuntime {
    /// 运行状态与当前运行的停止令牌由同一把锁保护，避免停止请求落到错误的运行上。
    state: Mutex<ScanRuntimeState>,
}

struct ScanRuntimeState {
    /// `true` 表示一个扫描已获准启动，直至其工作线程到达终态。
    running: bool,
    /// 当前扫描的停止令牌。每次成功 claim 都创建一个新的令牌。
    stop: Option<StopToken>,
}

impl ScanRuntime {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(ScanRuntimeState {
                running: false,
                stop: None,
            }),
        }
    }

    /// 启动扫描档案库任务：占用运行状态并创建本次令牌 → 推送状态 → 后台线程执行。
    pub(crate) fn start<W>(self: &Arc<Self>, handle: &AppHandle, worker_factory: impl FnOnce() -> W)
    where
        W: ScanWorker,
    {
        let Some(stop) = self.claim_start() else {
            warn!("扫描档案库任务正在运行中，忽略重复的启动请求");
            return;
        };
        info!("收到启动扫描档案库任务请求");
        let worker = worker_factory();
        self.emit_status(handle, None);

        let runtime = Arc::clone(self);
        let handle = handle.clone();
        thread::Builder::new()
            .name("oea-scan".to_string())
            .spawn(move || {
                let result = worker.run(stop);
                runtime.handle_worker_exit(&handle, result);
            })
            .expect("启动扫描档案库任务线程失败");
    }

    /// 请求停止扫描档案库任务。
    pub(crate) fn stop(&self) {
        let state = self.state.lock().unwrap();
        let Some(stop) = state.stop.as_ref() else {
            warn!("扫描档案库任务未在运行，忽略停止请求");
            return;
        };
        request_stop(stop);
        info!("收到停止请求，正在停止扫描档案库任务...");
    }

    /// 进程退出前请求停止，不改变原有的立即退出策略。
    pub(crate) fn request_stop_for_shutdown(&self) {
        let state = self.state.lock().unwrap();
        if let Some(stop) = state.stop.as_ref() {
            request_stop(stop);
        }
    }

    /// 读取当前扫描状态。
    pub(crate) fn status(&self) -> AppStatus {
        AppStatus {
            running: self.state.lock().unwrap().running,
            scan_error: None,
        }
    }

    fn claim_start(&self) -> Option<StopToken> {
        let mut state = self.state.lock().unwrap();
        if state.running {
            return None;
        }

        let stop = new_stop_token();
        state.running = true;
        state.stop = Some(Arc::clone(&stop));
        Some(stop)
    }

    /// 处理扫描终态：记录结果、释放运行标志并推送结束事件与空闲状态。
    fn handle_worker_exit(&self, handle: &AppHandle, result: ScanWorkerExit) {
        let ScanWorkerExit { reason, capture } = result;
        let public_outcome = reason.public_outcome();
        let scan_error = match reason {
            FinishReason::Completed => {
                info!("========== 扫描档案库任务执行完毕 ==========");
                None
            }
            FinishReason::Interrupted => {
                info!("扫描档案库任务已被用户停止");
                None
            }
            FinishReason::Failed(message) => {
                error!("{message}");
                Some(message)
            }
        };

        self.log_capture_summary(public_outcome, capture.as_ref());
        self.finish_run();
        self.emit_run_finished(
            handle,
            AutomationRunFinished::archive_scan(public_outcome, capture),
        );
        self.emit_status(handle, scan_error);
    }

    fn log_capture_summary(&self, outcome: AutomationOutcome, capture: Option<&CaptureSummary>) {
        let Some(capture) = capture else {
            return;
        };
        let calls = capture.calls;
        info!(
            "自动化统计捕获完成，耗时 {:.1} 秒",
            capture.elapsed.as_secs_f64()
        );
        debug!(
            task = "archiveScan",
            outcome = outcome.as_str(),
            elapsed_micros = capture.elapsed.as_micros(),
            screenshot_count = calls.screenshot,
            click_count = calls.click,
            press_key_count = calls.press_key,
            move_mouse_to_safe_position_count = calls.move_mouse_to_safe_position,
            find_template_count = calls.find_template,
            recognize_text_count = calls.recognize_text,
            sleep_count = calls.sleep,
            "自动化统计捕获明细"
        );
    }

    fn finish_run(&self) {
        let mut state = self.state.lock().unwrap();
        state.running = false;
        state.stop = None;
    }

    /// 向前端推送当前状态（running 标志 + 本次任务结束时的失败原因）。
    fn emit_status(&self, handle: &AppHandle, scan_error: Option<String>) {
        let status = AppStatus {
            running: self.state.lock().unwrap().running,
            scan_error,
        };
        if let Err(error) = handle.emit("app-status", &status) {
            error!("向前端推送状态失败: {error}");
        }
    }

    fn emit_run_finished(&self, handle: &AppHandle, event: AutomationRunFinished) {
        if let Err(error) = handle.emit("automation-run-finished", &event) {
            error!("向前端推送自动化结束事件失败: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::automation::{
        is_stop_requested,
        stats::counts::{CapabilityCallCounts, CaptureSummary},
    };

    use super::{AutomationOutcome, AutomationRunFinished, ScanRuntime};

    #[test]
    fn new_runtime_is_idle() {
        assert!(!ScanRuntime::new().status().running);
    }

    #[test]
    fn running_scan_rejects_a_second_claim_until_it_finishes() {
        let runtime = ScanRuntime::new();

        assert!(runtime.claim_start().is_some());
        assert!(runtime.claim_start().is_none());

        // 模拟工作线程处理终态后释放运行标志。
        runtime.finish_run();
        assert!(runtime.claim_start().is_some());
    }

    #[test]
    fn stop_request_is_ignored_while_idle_and_recorded_while_running() {
        let runtime = ScanRuntime::new();

        runtime.stop();
        assert!(!runtime.status().running);

        assert!(runtime.claim_start().is_some());
        runtime.stop();
        assert!(is_stop_requested(
            runtime.state.lock().unwrap().stop.as_ref().unwrap()
        ));
    }

    #[test]
    fn each_claim_gets_a_fresh_clear_stop_token() {
        let runtime = ScanRuntime::new();

        let first_stop = runtime.claim_start().unwrap();
        runtime.stop();
        assert!(is_stop_requested(&first_stop));

        runtime.finish_run();
        let second_stop = runtime.claim_start().unwrap();
        assert!(!is_stop_requested(&second_stop));
        assert!(!Arc::ptr_eq(&first_stop, &second_stop));
    }

    #[test]
    fn automation_run_finished_uses_the_frontend_wire_format() {
        let event = AutomationRunFinished::archive_scan(
            AutomationOutcome::Stopped,
            Some(CaptureSummary {
                elapsed: Duration::from_micros(1_234_567),
                calls: CapabilityCallCounts {
                    screenshot: 2,
                    click: 3,
                    press_key: 5,
                    move_mouse_to_safe_position: 7,
                    find_template: 11,
                    recognize_text: 13,
                    sleep: 17,
                },
            }),
        );

        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "task": "archiveScan",
                "outcome": "stopped",
                "capture": {
                    "elapsedMicros": 1_234_567,
                    "calls": {
                        "screenshot": 2,
                        "click": 3,
                        "pressKey": 5,
                        "moveMouseToSafePosition": 7,
                        "findTemplate": 11,
                        "recognizeText": 13,
                        "sleep": 17,
                    },
                },
            })
        );
    }

    #[test]
    fn automation_run_finished_has_no_capture_when_the_interval_never_started() {
        let event = AutomationRunFinished::archive_scan(AutomationOutcome::Failed, None);

        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "task": "archiveScan",
                "outcome": "failed",
                "capture": null,
            })
        );
    }
}
