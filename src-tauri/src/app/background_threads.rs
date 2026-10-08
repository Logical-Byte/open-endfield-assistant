//! 应用常驻后台线程的创建与有序释放。

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

use anyhow::Result;
use tauri::AppHandle;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;

use crate::{logger::LogEntry, platform};

use super::{automation_events, frontend_forwarders, hotkeys};

/// Tauri 托管的应用常驻线程所有者。
pub(super) struct BackgroundThreads {
    inner: Mutex<Option<Inner>>,
}

struct Inner {
    frontend_event_sink: Arc<automation_events::TauriEventSink>,
    stop: Arc<AtomicBool>,
    keyboard_hook: platform::hotkey::KeyboardHookGuard,
    hotkey_thread: JoinHandle<()>,
    log_thread: JoinHandle<()>,
    logger_guard: WorkerGuard,
}

impl BackgroundThreads {
    /// 接管已启动的自动化事件线程，并启动键盘监听、热键分发和日志转发。
    ///
    /// 事件线程先用于组装自动化运行时。至此所有常驻线程及保活资源统一归本对象管理。
    pub(super) fn start(
        app_handle: &AppHandle,
        frontend_event_sink: Arc<automation_events::TauriEventSink>,
        logger_guard: WorkerGuard,
        log_rx: mpsc::Receiver<LogEntry>,
    ) -> Result<Self> {
        let oea_window = platform::window::get_app_window(app_handle)?;
        let foreground = platform::window::ForegroundGuard::new(oea_window);
        let (hotkey_rx, keyboard_hook) = platform::hotkey::listen()?;
        let stop = Arc::new(AtomicBool::new(false));

        let hotkey_thread =
            hotkeys::spawn_dispatcher(hotkey_rx, Arc::clone(&stop), foreground, app_handle.clone());
        let log_thread =
            frontend_forwarders::spawn_log_forwarder(log_rx, Arc::clone(&stop), app_handle.clone());

        Ok(Self {
            inner: Mutex::new(Some(Inner {
                frontend_event_sink,
                stop,
                keyboard_hook,
                hotkey_thread,
                log_thread,
                logger_guard,
            })),
        })
    }

    /// 通知全部常驻线程停止，阻塞等待它们退出，并在最后刷新文件日志。
    pub(super) fn shutdown(&self) {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(inner) = inner else {
            return;
        };

        // 保持原退出顺序：先排空自动化事件，再停止热键和日志转发，最后刷新文件日志。
        inner.frontend_event_sink.shutdown();
        inner.stop.store(true, Ordering::Relaxed);

        if let Err(error) = inner.keyboard_hook.shutdown() {
            error!(%error, "关闭键盘监听线程失败");
        }
        join_thread("热键动作分发", inner.hotkey_thread);
        join_thread("日志前端转发", inner.log_thread);

        info!("应用常驻后台线程已全部停止");
        drop(inner.logger_guard);
    }
}

impl Drop for BackgroundThreads {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn join_thread(name: &str, thread: JoinHandle<()>) {
    if thread.join().is_err() {
        error!(thread = name, "应用常驻后台线程发生 panic");
    }
}
