//! 真实游戏扫描的工作线程内容。
//!
//! 平台窗口、会话、任务与提示音都留在这里；[`crate::automation::scan_runtime::ScanRuntime`]
//! 只管理线程生命周期。

use std::sync::{Arc, Mutex};

use tracing::warn;

use crate::{
    app_paths::AppPaths,
    automation::{
        AutomationStopped, StopToken, is_stop_requested,
        scan_runtime::{ScanOutcome, ScanRunResult},
        session::Session,
        stats::counts::Capture,
    },
    config::OeaConfig,
    data::AppData,
    navigation::Navigator,
    ocr::OcrEngine,
    platform,
    task::{
        archive_scan::{ArchiveScanTask, ScanReporter},
        run_task,
    },
};

/// 一次真实扫描所需的协作者，由控制器在任务获准启动后创建。
pub(crate) struct LiveScanWorker {
    oea_config: OeaConfig,
    ocr: Arc<Mutex<OcrEngine>>,
    navigator: Arc<Navigator>,
    app_data: Arc<AppData>,
    reporter: ScanReporter,
}

impl LiveScanWorker {
    pub(crate) fn new(
        oea_config: OeaConfig,
        ocr: Arc<Mutex<OcrEngine>>,
        navigator: Arc<Navigator>,
        app_data: Arc<AppData>,
        reporter: ScanReporter,
    ) -> Self {
        Self {
            oea_config,
            ocr,
            navigator,
            app_data,
            reporter,
        }
    }

    pub(crate) fn run(self, stop: StopToken) -> ScanRunResult {
        let result = self.run_task(stop);
        let final_sound = match &result.outcome {
            ScanOutcome::Completed => ScanSound::Enable,
            ScanOutcome::Stopped | ScanOutcome::Failed(_) => ScanSound::Disable,
        };
        self.play_scan_sound(final_sound);
        result
    }

    fn run_task(&self, stop: StopToken) -> ScanRunResult {
        // 任务开始时才连接游戏。
        let mut session = match Session::connect(&self.ocr, Arc::clone(&stop)) {
            Ok(session) => session,
            Err(_error) if is_stop_requested(&stop) => {
                return ScanRunResult::without_capture(ScanOutcome::Stopped);
            }
            Err(error) => {
                return ScanRunResult::without_capture(ScanOutcome::Failed(format!(
                    "连接游戏失败: {error:#}"
                )));
            }
        };

        // 停止请求可能发生在连接过程中，不能让它被清除或跳过。
        if is_stop_requested(&stop) {
            return ScanRunResult::without_capture(ScanOutcome::Stopped);
        }

        // 扫描任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）。
        if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
            warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
        }

        // 启动检查通过、任务真正开始执行前播放提示音。
        self.play_scan_sound(ScanSound::Enable);

        let task = ArchiveScanTask::new(self.reporter.clone(), self.app_data.archive_titles());
        let mut captured = Capture::new(&mut session);
        let result = run_task(&task, &mut captured, &self.navigator);
        let capture = captured.finish();

        let outcome = match result {
            Ok(()) => ScanOutcome::Completed,
            Err(error) if error.downcast_ref::<AutomationStopped>().is_some() => {
                ScanOutcome::Stopped
            }
            Err(error) => ScanOutcome::Failed(format!("扫描档案库任务执行失败: {error:#}")),
        };
        ScanRunResult {
            outcome,
            capture: Some(capture),
        }
    }

    /// 播放扫描提示音（音量取本次配置快照）。
    fn play_scan_sound(&self, sound: ScanSound) {
        let app_paths = match AppPaths::new() {
            Ok(app_paths) => app_paths,
            Err(error) => {
                warn!("无法解析扫描提示音资源: {error}");
                return;
            }
        };
        let path = match app_paths.resolve_resource_file(sound.relative_path()) {
            Ok(path) => path,
            Err(error) => {
                warn!("无法解析扫描提示音资源: {error:#}");
                return;
            }
        };
        platform::sound::play_wav(&path, self.oea_config.sound_volume);
    }
}

#[derive(Debug, Clone, Copy)]
enum ScanSound {
    Enable,
    Disable,
}

impl ScanSound {
    const fn relative_path(self) -> &'static str {
        match self {
            Self::Enable => "sounds/enable.wav",
            Self::Disable => "sounds/disable.wav",
        }
    }
}
