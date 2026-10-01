//! 自动化事件到 `Tauri` 前端事件的 adapter。

use std::{
    sync::{Mutex, mpsc},
    thread::{self, JoinHandle},
};

use tauri::{AppHandle, Emitter};
use tracing::error;

use crate::automation;

pub(super) struct TauriEventSink {
    inner: Mutex<ThreadState>,
}

struct ThreadState {
    sender: Option<mpsc::Sender<EventThreadCommand>>,
    thread: Option<JoinHandle<()>>,
}

enum EventThreadCommand {
    Emit(automation::Event),
    Shutdown,
}

impl TauriEventSink {
    pub(super) fn start(app_handle: AppHandle) -> std::io::Result<Self> {
        // 使用无界通道，让 `EventSink::publish` 只负责快速接受事件。
        let (sender, receiver) = mpsc::channel();
        let thread = run_event_thread(app_handle, receiver)?;

        Ok(Self {
            inner: Mutex::new(ThreadState {
                sender: Some(sender),
                thread: Some(thread),
            }),
        })
    }

    pub(super) fn shutdown(&self) {
        let (sender, thread) = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (inner.sender.take(), inner.thread.take())
        };

        if let Some(sender) = sender
            && sender.send(EventThreadCommand::Shutdown).is_err()
        {
            error!("关闭自动化事件线程失败: 接收端已断开");
        }
        if let Some(thread) = thread
            && thread.join().is_err()
        {
            error!("自动化事件线程发生 panic");
        }
    }
}

impl automation::EventSink for TauriEventSink {
    fn publish(&self, event: automation::Event) {
        let disconnected = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(sender) = inner.sender.as_ref() else {
                return;
            };
            if sender.send(EventThreadCommand::Emit(event)).is_err() {
                inner.sender.take();
                true
            } else {
                false
            }
        };

        if disconnected {
            error!("向自动化事件线程发送事件失败: 接收端已断开");
        }
    }
}

impl Drop for TauriEventSink {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_event_thread(
    app_handle: AppHandle,
    receiver: mpsc::Receiver<EventThreadCommand>,
) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("oea-automation-events".to_string())
        .spawn(move || {
            while let Ok(command) = receiver.recv() {
                match command {
                    EventThreadCommand::Emit(event) => {
                        if let Err(error) = emit_event(&app_handle, event) {
                            error!("向前端推送自动化事件失败: {error}");
                        }
                    }
                    EventThreadCommand::Shutdown => break,
                }
            }
        })
}

fn emit_event(app_handle: &AppHandle, event: automation::Event) -> tauri::Result<()> {
    match event {
        automation::Event::StatusChanged(status) => {
            app_handle.emit("automation-status-changed", status)
        }
        automation::Event::ArchiveScanResult(result) => app_handle.emit("scan-result", result),
    }
}
