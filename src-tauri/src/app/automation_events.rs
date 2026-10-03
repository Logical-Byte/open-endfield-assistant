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
    EmitStatus(automation::Status),
    EmitRun {
        run_id: u32,
        event: automation::Event,
    },
    Shutdown,
}

impl TauriEventSink {
    pub(super) fn start(app_handle: AppHandle) -> std::io::Result<Self> {
        // 使用无界通道，让事件发布只负责快速接受事件。
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
    fn send(&self, command: EventThreadCommand) {
        let disconnected = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(sender) = inner.sender.as_ref() else {
                return;
            };
            if sender.send(command).is_err() {
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

impl automation::RuntimeEventSink for TauriEventSink {
    fn publish_status(&self, status: automation::Status) {
        self.send(EventThreadCommand::EmitStatus(status));
    }

    fn publish_run_event(&self, run_id: u32, event: automation::Event) {
        self.send(EventThreadCommand::EmitRun { run_id, event });
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
                let result = match command {
                    EventThreadCommand::EmitStatus(status) => {
                        app_handle.emit("automation-status-changed", status)
                    }
                    EventThreadCommand::EmitRun { run_id, event } => {
                        emit_run_event(&app_handle, run_id, event)
                    }
                    EventThreadCommand::Shutdown => break,
                };
                if let Err(error) = result {
                    error!("向前端推送自动化事件失败: {error}");
                }
            }
        })
}

fn emit_run_event(
    app_handle: &AppHandle,
    run_id: u32,
    event: automation::Event,
) -> tauri::Result<()> {
    match event {
        automation::Event::ArchiveItemScanned(payload) => app_handle.emit(
            "archive-item-scanned",
            automation::RunEvent { run_id, payload },
        ),
        automation::Event::EssenceItemScanned(payload) => app_handle.emit(
            "essence-item-scanned",
            automation::RunEvent { run_id, payload },
        ),
    }
}
