use std::sync::{Arc, Mutex};

use tracing::warn;

use crate::{
    automation::{
        AutomationStopped, EventSink, StopToken, is_stop_requested,
        runtime::{FinishReason, Worker, WorkerExit},
        session::Session,
        stats::counts::Capture,
    },
    essence, platform, settings, vision,
};

use super::workflow;

pub(crate) struct EssenceScanWorker {
    settings: settings::OeaSettings,
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
}

impl EssenceScanWorker {
    pub(crate) fn new(
        settings: settings::OeaSettings,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    ) -> Self {
        Self { settings, ocr }
    }
}

impl Worker for EssenceScanWorker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        if let Err(error) = self
            .settings
            .essence_scan
            .validate(essence::Catalog::bundled())
        {
            return WorkerExit::without_capture(FinishReason::Failed(format!(
                "基质扫描设置无效: {error:#}"
            )));
        }
        let mut session = match Session::connect(&self.ocr, Arc::clone(&stop)) {
            Ok(session) => session,
            Err(_) if is_stop_requested(&stop) => {
                return WorkerExit::without_capture(FinishReason::Stopped);
            }
            Err(error) => {
                return WorkerExit::without_capture(FinishReason::Failed(format!(
                    "连接游戏失败: {error:#}"
                )));
            }
        };
        if is_stop_requested(&stop) {
            return WorkerExit::without_capture(FinishReason::Stopped);
        }
        let (width, height) = session.client_size();
        if (width, height) != (1280, 720) {
            return WorkerExit::without_capture(FinishReason::Failed(format!(
                "基质扫描仅支持 1280×720 游戏窗口，当前为 {width}×{height}"
            )));
        }
        if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
            warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
        }

        let mut captured = Capture::new(&mut session);
        let result = workflow::scan(&mut captured, &self.settings.essence_scan, events.as_ref());
        let capture = captured.finish();
        let reason = match result {
            Ok(()) => FinishReason::Completed,
            Err(error) if error.downcast_ref::<AutomationStopped>().is_some() => {
                FinishReason::Stopped
            }
            Err(error) => FinishReason::Failed(format!("基质扫描失败: {error:#}")),
        };
        WorkerExit::with_capture(reason, capture)
    }
}
