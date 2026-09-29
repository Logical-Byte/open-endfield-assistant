//! 自动化事件到 Tauri 前端事件的 adapter。

use tauri::{AppHandle, Emitter};
use tracing::error;

use crate::automation::{Event, EventSink};

pub(super) struct TauriEventSink {
    app_handle: AppHandle,
}

impl TauriEventSink {
    pub(super) fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl EventSink for TauriEventSink {
    fn publish(&self, event: Event) {
        let result = match event {
            Event::StatusChanged(status) => self.app_handle.emit("automation-status", status),
            Event::RunFinished(finished) => {
                self.app_handle.emit("automation-run-finished", finished)
            }
            Event::ArchiveScanResult(result) => self.app_handle.emit("scan-result", result),
        };

        if let Err(error) = result {
            error!("向前端推送自动化事件失败: {error}");
        }
    }
}
