//! 扫描档案库任务的运行时。
//!
//! [`ScanRuntime`] 管理扫描的运行状态、停止令牌与工作线程。
//! 真实游戏操作由调用方提供的工作者执行。

use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

use crate::automation::{StopToken, new_stop_token, request_stop, stats::counts::CaptureSummary};

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

/// 一次扫描运行的终态。
pub(crate) enum ScanOutcome {
    Completed,
    Stopped,
    Failed(String),
}

/// 工作者交还给运行时的终态与 capability 捕获结果。
pub(crate) struct ScanWorkerResult {
    pub(crate) outcome: ScanOutcome,
    pub(crate) capture: Option<CaptureSummary>,
}

impl ScanWorkerResult {
    pub(crate) fn without_capture(outcome: ScanOutcome) -> Self {
        Self {
            outcome,
            capture: None,
        }
    }

    pub(crate) fn with_capture(outcome: ScanOutcome, capture: CaptureSummary) -> Self {
        Self {
            outcome,
            capture: Some(capture),
        }
    }
}

/// 一次扫描工作线程的执行内容。按值接收工作者，避免同一次运行重复执行。
pub(crate) trait ScanWorker: Send + 'static {
    fn run(self, stop: StopToken) -> ScanWorkerResult;
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
                runtime.handle_run_exit(&handle, result);
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

    /// 处理扫描终态：记录结果、释放运行标志并推送空闲状态。
    fn handle_run_exit(&self, handle: &AppHandle, result: ScanWorkerResult) {
        let ScanWorkerResult { outcome, capture } = result;
        if let Some(summary) = capture {
            let calls = summary.calls;
            info!(
                "扫描档案库工作流统计：耗时 {:.1} 秒，截图 {} 次，点击 {} 次，按键 {} 次，\
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

        let scan_error = match outcome {
            ScanOutcome::Completed => {
                info!("========== 扫描档案库任务执行完毕 ==========");
                None
            }
            ScanOutcome::Stopped => {
                info!("扫描档案库任务已被用户停止");
                None
            }
            ScanOutcome::Failed(message) => {
                error!("{message}");
                Some(message)
            }
        };

        self.finish_run();
        self.emit_status(handle, scan_error);
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::automation::is_stop_requested;

    use super::ScanRuntime;

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
}
