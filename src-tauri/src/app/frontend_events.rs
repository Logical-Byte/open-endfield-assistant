//! 自动化事件到 Tauri 前端事件的 adapter。

use tauri::{AppHandle, Emitter};
use tracing::error;

use crate::automation;

pub(super) struct TauriEventSink {
    app_handle: AppHandle,
}

impl TauriEventSink {
    pub(super) fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl automation::EventSink for TauriEventSink {
    fn publish(&self, event: automation::Event) {
        let result = match event {
            automation::Event::StatusChanged(status) => {
                self.app_handle.emit("automation-status", status)
            }
            automation::Event::RunFinished(finished) => {
                self.app_handle.emit("automation-run-finished", finished)
            }
            automation::Event::ArchiveScanResult(result) => {
                self.app_handle.emit("scan-result", result)
            }
        };

        if let Err(error) = result {
            error!("向前端推送自动化事件失败: {error}");
        }
    }
}
