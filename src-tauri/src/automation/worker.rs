//! 真实游戏扫描的工作线程内容。
//!
//! 游戏会话、窗口操作、档案任务和提示音集中在这里；
//! [`crate::automation::scan_runtime::ScanRuntime`] 只管理运行生命周期。

use std::sync::{Arc, Mutex};

use tracing::warn;

use crate::{
    app_paths::AppPaths,
    automation::{
        AutomationStopped, StopToken, is_stop_requested,
        scan_runtime::{ScanOutcome, ScanWorker},
        session::Session,
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

    fn run_task(&self, stop: StopToken) -> ScanOutcome {
        // 连接游戏可能耗时，所以留在工作线程中。
        let mut session = match Session::connect(&self.ocr, Arc::clone(&stop)) {
            Ok(session) => session,
            Err(_error) if is_stop_requested(&stop) => return ScanOutcome::Stopped,
            Err(error) => return ScanOutcome::Failed(format!("连接游戏失败: {error:#}")),
        };

        // 停止请求可能发生在连接过程中，不能让它被清除或跳过。
        if is_stop_requested(&stop) {
            return ScanOutcome::Stopped;
        }

        // 扫描档案库任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）。
        if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
            warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
        }

        // 启动检查通过、任务真正开始执行前播放 enable 提示音。
        self.play_scan_sound(ScanSound::Enable);

        let task = ArchiveScanTask::new(self.reporter.clone(), self.app_data.archive_titles());
        let result = run_task(&task, &mut session, &self.navigator);

        match result {
            Ok(()) => ScanOutcome::Completed,
            Err(error) if error.downcast_ref::<AutomationStopped>().is_some() => {
                ScanOutcome::Stopped
            }
            Err(error) => ScanOutcome::Failed(format!("扫描档案库任务执行失败: {error:#}")),
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

impl ScanWorker for LiveScanWorker {
    fn run(self, stop: StopToken) -> ScanOutcome {
        let outcome = self.run_task(stop);
        self.play_scan_sound(match &outcome {
            ScanOutcome::Completed => ScanSound::Enable,
            ScanOutcome::Stopped | ScanOutcome::Failed(_) => ScanSound::Disable,
        });
        outcome
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
