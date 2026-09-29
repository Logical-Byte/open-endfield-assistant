//! 真实游戏扫描的工作线程内容。
//!
//! 游戏会话、窗口操作、档案扫描工作流和提示音集中在这里；
//! [`crate::automation::runtime::Runtime`] 只管理运行生命周期。

use std::sync::{Arc, Mutex};

use tracing::warn;

use crate::{
    app_paths::AppPaths,
    automation::{
        AutomationStopped, EventSink, StopToken, is_stop_requested,
        runtime::{FinishReason, Worker, WorkerExit},
        session::Session,
        stats::counts::Capture,
    },
    data::AppData,
    navigation::Navigator,
    platform, settings, vision,
};

use super::{reporting::ScanReporter, workflow::ArchiveScanner};

/// 一次真实扫描所需的协作者，由控制器在任务获准启动后创建。
pub(crate) struct ArchiveScanWorker {
    settings: settings::OeaSettings,
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    navigator: Arc<Navigator>,
    app_data: Arc<AppData>,
}

impl ArchiveScanWorker {
    pub(crate) fn new(
        settings: settings::OeaSettings,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
        navigator: Arc<Navigator>,
        app_data: Arc<AppData>,
    ) -> Self {
        Self {
            settings,
            ocr,
            navigator,
            app_data,
        }
    }

    fn run_scan(&self, stop: StopToken, reporter: ScanReporter) -> WorkerExit {
        // 连接游戏可能耗时，所以留在工作线程中。
        let mut session = match Session::connect(&self.ocr, Arc::clone(&stop)) {
            Ok(session) => session,
            Err(_error) if is_stop_requested(&stop) => {
                return WorkerExit::without_capture(FinishReason::Stopped);
            }
            Err(error) => {
                return WorkerExit::without_capture(FinishReason::Failed(format!(
                    "连接游戏失败: {error:#}"
                )));
            }
        };

        // 停止请求可能发生在连接过程中，不能让它被清除或跳过。
        if is_stop_requested(&stop) {
            return WorkerExit::without_capture(FinishReason::Stopped);
        }

        // 扫描档案库任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）。
        if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
            warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
        }

        // 启动检查通过、任务真正开始执行前播放 enable 提示音。
        self.play_scan_sound(ScanSound::Enable);

        let scanner = ArchiveScanner::new(reporter, self.app_data.archive_titles());
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
        WorkerExit::with_capture(reason, capture)
    }

    /// 播放扫描提示音（音量取本次设置快照）。
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
        platform::sound::play_wav(&path, self.settings.sound_volume);
    }
}

impl Worker for ArchiveScanWorker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        let result = self.run_scan(stop, ScanReporter::new(events));
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
