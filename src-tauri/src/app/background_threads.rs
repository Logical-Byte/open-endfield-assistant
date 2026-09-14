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

use crate::{logger::LogEntry, platform, task::archive_scan::ScanResult};

use super::{frontend_forwarders, hotkeys};

/// Tauri 托管的应用常驻线程所有者。
pub(super) struct BackgroundThreads {
    inner: Mutex<Option<Inner>>,
}

struct Inner {
    stop: Arc<AtomicBool>,
    keyboard_hook: platform::hotkey::KeyboardHookGuard,
    hotkey_thread: JoinHandle<()>,
    log_thread: JoinHandle<()>,
    scan_result_thread: JoinHandle<()>,
    logger_guard: WorkerGuard,
}

impl BackgroundThreads {
    /// 启动键盘监听、热键分发和前端事件转发线程。
    pub(super) fn start(
        app_handle: &AppHandle,
        logger_guard: WorkerGuard,
        log_rx: mpsc::Receiver<LogEntry>,
        scan_result_rx: mpsc::Receiver<ScanResult>,
    ) -> Result<Self> {
        let oea_window = platform::window::get_app_window(app_handle)?;
        let foreground = platform::window::ForegroundGuard::new(oea_window);
        let (hotkey_rx, keyboard_hook) = platform::hotkey::listen()?;
        let stop = Arc::new(AtomicBool::new(false));

        let hotkey_thread =
            hotkeys::spawn_dispatcher(hotkey_rx, Arc::clone(&stop), foreground, app_handle.clone());
        let scan_result_thread = frontend_forwarders::spawn_scan_result_forwarder(
            scan_result_rx,
            Arc::clone(&stop),
            app_handle.clone(),
        );
        let log_thread =
            frontend_forwarders::spawn_log_forwarder(log_rx, Arc::clone(&stop), app_handle.clone());

        Ok(Self {
            inner: Mutex::new(Some(Inner {
                stop,
                keyboard_hook,
                hotkey_thread,
                log_thread,
                scan_result_thread,
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

        inner.stop.store(true, Ordering::Relaxed);

        if let Err(error) = inner.keyboard_hook.shutdown() {
            error!(%error, "关闭键盘监听线程失败");
        }
        join_thread("热键动作分发", inner.hotkey_thread);
        join_thread("扫描结果前端转发", inner.scan_result_thread);
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
