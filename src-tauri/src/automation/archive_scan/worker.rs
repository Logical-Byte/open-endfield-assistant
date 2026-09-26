//! 真实游戏扫描的工作线程内容。
//!
//! 游戏会话、窗口操作、档案扫描工作流和提示音集中在这里；
//! [`crate::automation::scan_runtime::ScanRuntime`] 只管理运行生命周期。

use std::sync::{Arc, Mutex, mpsc};

use tracing::warn;

use crate::{
    app_paths::AppPaths,
    automation::{
        AutomationStopped, StopToken, is_stop_requested,
        scan_runtime::{FinishReason, ScanWorker, ScanWorkerExit},
        session::Session,
        stats::counts::Capture,
    },
    config::OeaConfig,
    data::AppData,
    navigation::Navigator,
    ocr::OcrEngine,
    platform,
};

use super::{
    reporting::{ScanReporter, ScanResult},
    workflow::ArchiveScanner,
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
        scan_tx: mpsc::Sender<ScanResult>,
    ) -> Self {
        Self {
            oea_config,
            ocr,
            navigator,
            app_data,
            reporter: ScanReporter::new(scan_tx),
        }
    }

    fn run_scan(&self, stop: StopToken) -> ScanWorkerExit {
        // 连接游戏可能耗时，所以留在工作线程中。
        let mut session = match Session::connect(&self.ocr, Arc::clone(&stop)) {
            Ok(session) => session,
            Err(_error) if is_stop_requested(&stop) => {
                return ScanWorkerExit::without_capture(FinishReason::Stopped);
            }
            Err(error) => {
                return ScanWorkerExit::without_capture(FinishReason::Failed(format!(
                    "连接游戏失败: {error:#}"
                )));
            }
        };

        // 停止请求可能发生在连接过程中，不能让它被清除或跳过。
        if is_stop_requested(&stop) {
            return ScanWorkerExit::without_capture(FinishReason::Stopped);
        }

        // 扫描档案库任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）。
        if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
            warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
        }

        // 启动检查通过、任务真正开始执行前播放 enable 提示音。
        self.play_scan_sound(ScanSound::Enable);

        let scanner = ArchiveScanner::new(self.reporter.clone(), self.app_data.archive_titles());
        let mut captured = Capture::new(&mut session);
        let result = scanner.run(&mut captured, &self.navigator);
        let capture = captured.finish();

        let reason = match result {
            Ok(()) => FinishReason::Completed,
            Err(error) if error.downcast_ref::<AutomationStopped>().is_some() => {
                FinishReason::Stopped
            }
            Err(error) => FinishReason::Failed(format!("扫描档案库任务执行失败: {error:#}")),
        };
        ScanWorkerExit::with_capture(reason, capture)
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

impl ScanWorker for LiveScanWorker {
    fn run(self, stop: StopToken) -> ScanWorkerExit {
        let result = self.run_scan(stop);
        self.play_scan_sound(match &result.reason {
            FinishReason::Completed => ScanSound::Enable,
            FinishReason::Stopped | FinishReason::Failed(_) => ScanSound::Disable,
        });
        result
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
