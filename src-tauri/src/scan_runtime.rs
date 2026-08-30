//! 扫描档案库任务的运行时。
//!
//! [`ScanRuntime`] 只拥有一次扫描任务的生命周期状态。每次启动由
//! [`ScanRunContext`] 带入本次运行所需的应用协作者，运行时不会把这些协作者
//! 保留为状态。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

use crate::{
    app_paths::AppPaths,
    config::OeaConfig,
    connect::connect_to_game,
    ocr::OcrEngine,
    scene::SceneManager,
    sound,
    task::{
        TaskStopped,
        archive_scan::{ArchiveScanTask, CorrectionIndex, ScanReporter},
        run_task,
    },
    windows_ops,
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

/// 一次扫描运行所需的应用协作者。
///
/// 由 [`crate::controller::Controller`] 在任务被接受后创建，并移交给唯一的扫描线程。
pub(crate) struct ScanRunContext {
    app_path: AppPaths,
    oea_config: Arc<Mutex<OeaConfig>>,
    ocr: Arc<Mutex<OcrEngine>>,
    scenes: Arc<SceneManager>,
    correction: Arc<CorrectionIndex>,
    reporter: ScanReporter,
    handle: AppHandle,
}

impl ScanRunContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        app_path: AppPaths,
        oea_config: Arc<Mutex<OeaConfig>>,
        ocr: Arc<Mutex<OcrEngine>>,
        scenes: Arc<SceneManager>,
        correction: Arc<CorrectionIndex>,
        reporter: ScanReporter,
        handle: AppHandle,
    ) -> Self {
        Self {
            app_path,
            oea_config,
            ocr,
            scenes,
            correction,
            reporter,
            handle,
        }
    }
}

/// 一次扫描运行的终态。
enum ScanOutcome {
    Completed,
    Stopped,
    Failed(String),
}

/// 扫描档案库任务的生命周期状态。
pub(crate) struct ScanRuntime {
    /// 停止请求标志：`true` 表示当前扫描应尽快停止。
    ///
    /// [`Self::stop`] 和 [`Self::request_stop_for_shutdown`] 写入 `true`；工作线程在
    /// 成功连接游戏后通过 [`crate::session::Session::reset_stop`] 写回 `false`。
    /// 一次扫描的 [`crate::session::Session`] 在每个游戏操作前读取。
    stop: Arc<AtomicBool>,
    /// 运行标志：`true` 表示一个扫描已获准启动，直至其工作线程到达终态。
    ///
    /// [`Self::claim_start`] 用 CAS 写入 `true`；状态查询、停止与切换操作读取；
    /// 工作线程的 [`Self::handle_run_exit`] 在成功、停止或失败后写回 `false`。
    running: AtomicBool,
}

impl ScanRuntime {
    pub(crate) fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            running: AtomicBool::new(false),
        }
    }

    /// 启动扫描档案库任务：CAS 占用运行标志 → 推送状态 → 后台线程执行。
    pub(crate) fn start(self: &Arc<Self>, context_factory: impl FnOnce() -> ScanRunContext) {
        if !self.claim_start() {
            warn!("扫描档案库任务正在运行中，忽略重复的启动请求");
            return;
        }
        info!("收到启动扫描档案库任务请求");
        let context = context_factory();
        self.emit_status(&context.handle, None);

        let runtime = Arc::clone(self);
        thread::Builder::new()
            .name("oea-scan".to_string())
            .spawn(move || {
                let outcome = runtime.run(&context);
                runtime.handle_run_exit(&context, outcome);
            })
            .expect("启动扫描档案库任务线程失败");
    }

    /// 请求停止扫描档案库任务。
    pub(crate) fn stop(&self) {
        if !self.running.load(Ordering::Relaxed) {
            warn!("扫描档案库任务未在运行，忽略停止请求");
            return;
        }
        self.request_stop_for_shutdown();
        info!("收到停止请求，正在停止扫描档案库任务...");
    }

    /// 进程退出前请求停止，不改变原有的立即退出策略。
    pub(crate) fn request_stop_for_shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// 读取当前扫描状态。
    pub(crate) fn status(&self) -> AppStatus {
        AppStatus {
            running: self.running.load(Ordering::Relaxed),
            scan_error: None,
        }
    }

    fn claim_start(&self) -> bool {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn run(&self, context: &ScanRunContext) -> ScanOutcome {
        // 任务开始时才连接游戏
        let mut session = match connect_to_game(
            &context.ocr,
            &context.app_path.templates_dir(),
            Arc::clone(&self.stop),
        ) {
            Ok(session) => session,
            Err(error) => {
                return ScanOutcome::Failed(format!("连接游戏失败: {error:#}"));
            }
        };

        // 扫描档案库任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）
        if let Err(error) = windows_ops::window::ensure_foreground_and_topmost(session.hwnd) {
            warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
        }

        // 清除上一次可能残留的停止信号
        session.reset_stop();

        // 启动检查通过、任务真正开始执行前播放 enable 提示音
        // （避免"启动后立即失败"时 enable/disable 两个音效同时播放）
        self.play_scan_sound(context, true);

        // 执行扫描档案库任务（阻塞，期间任务内部轮询停止标志）
        let task = ArchiveScanTask::new(context.reporter.clone(), Arc::clone(&context.correction));
        let result = run_task(&task, &mut session, &context.scenes);

        match result {
            Ok(()) => ScanOutcome::Completed,
            Err(error) if error.downcast_ref::<TaskStopped>().is_some() => ScanOutcome::Stopped,
            Err(error) => ScanOutcome::Failed(format!("扫描档案库任务执行失败: {error:#}")),
        }
    }

    /// 处理扫描终态：记录结果、播放提示音、释放运行标志并推送空闲状态。
    fn handle_run_exit(&self, context: &ScanRunContext, outcome: ScanOutcome) {
        let scan_error = match outcome {
            ScanOutcome::Completed => {
                info!("========== 扫描档案库任务执行完毕 ==========");
                self.play_scan_sound(context, true);
                None
            }
            ScanOutcome::Stopped => {
                info!("扫描档案库任务已被用户停止");
                self.play_scan_sound(context, false);
                None
            }
            ScanOutcome::Failed(message) => {
                error!("{message}");
                self.play_scan_sound(context, false);
                Some(message)
            }
        };

        self.running.store(false, Ordering::Relaxed);
        self.emit_status(&context.handle, scan_error);
    }

    /// 播放扫描提示音（音量取配置；开始/自然完成播 enable，失败/被停止播 disable）。
    fn play_scan_sound(&self, context: &ScanRunContext, enable: bool) {
        let volume = context
            .oea_config
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .sound_volume;
        let name = if enable { "enable.wav" } else { "disable.wav" };
        let path = context.app_path.resources_dir().join("sounds").join(name);
        sound::play_wav(&path, volume);
    }

    /// 向前端推送当前状态（running 标志 + 本次任务结束时的失败原因）。
    fn emit_status(&self, handle: &AppHandle, scan_error: Option<String>) {
        let status = AppStatus {
            running: self.running.load(Ordering::Relaxed),
            scan_error,
        };
        if let Err(error) = handle.emit("app-status", &status) {
            error!("向前端推送状态失败: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::ScanRuntime;

    #[test]
    fn new_runtime_is_idle() {
        assert!(!ScanRuntime::new().status().running);
    }

    #[test]
    fn running_scan_rejects_a_second_claim_until_it_finishes() {
        let runtime = ScanRuntime::new();

        assert!(runtime.claim_start());
        assert!(!runtime.claim_start());

        // 模拟工作线程处理终态后释放运行标志。
        runtime.running.store(false, Ordering::Relaxed);
        assert!(runtime.claim_start());
    }

    #[test]
    fn stop_request_is_ignored_while_idle_and_recorded_while_running() {
        let runtime = ScanRuntime::new();

        runtime.stop();
        assert!(!runtime.stop.load(Ordering::Relaxed));

        assert!(runtime.claim_start());
        runtime.stop();
        assert!(runtime.stop.load(Ordering::Relaxed));
    }
}
