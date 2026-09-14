//! 应用常驻后台线程的创建与有序释放。

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;

use crate::{controller::Controller, logger::LogEntry, platform, task::archive_scan::ScanResult};

const RECEIVE_TIMEOUT: Duration = Duration::from_millis(100);

/// 引号 `'`：切换扫描档案库任务。
const TOGGLE_MAIN_TASK_HOTKEY: platform::hotkey::KeyEvent = platform::hotkey::KeyEvent {
    vk: platform::hotkey::OEM_7_KEY,
    down: true,
    modifiers: 0,
};

/// Alt+Delete：退出应用。
const EXIT_HOTKEY: platform::hotkey::KeyEvent = platform::hotkey::KeyEvent {
    vk: platform::hotkey::DELETE_KEY,
    down: true,
    modifiers: platform::hotkey::ALT_MODIFIER,
};

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
            spawn_hotkey_loop(hotkey_rx, Arc::clone(&stop), foreground, app_handle.clone());
        let scan_result_thread =
            spawn_scan_result_loop(scan_result_rx, Arc::clone(&stop), app_handle.clone());
        let log_thread = spawn_log_loop(log_rx, Arc::clone(&stop), app_handle.clone());

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

fn spawn_hotkey_loop(
    rx: mpsc::Receiver<platform::hotkey::KeyEvent>,
    stop: Arc<AtomicBool>,
    foreground: platform::window::ForegroundGuard,
    app_handle: AppHandle,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("oea-hotkey".to_string())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let key_event = match rx.recv_timeout(RECEIVE_TIMEOUT) {
                    Ok(key_event) => key_event,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };

                if key_event == EXIT_HOTKEY {
                    let controller = app_handle.state::<Controller>();
                    controller.quit(&app_handle);
                } else if key_event == TOGGLE_MAIN_TASK_HOTKEY {
                    if foreground.is_foreground_eligible() {
                        let controller = app_handle.state::<Controller>();
                        controller.toggle_scan(&app_handle);
                    } else {
                        debug!("前台窗口不是终末地或者 OEA，忽略热键");
                    }
                }
            }
        })
        .expect("启动热键动作分发线程失败")
}

fn spawn_log_loop(
    rx: mpsc::Receiver<LogEntry>,
    stop: Arc<AtomicBool>,
    app_handle: AppHandle,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("oea-log".to_string())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let log_entry = match rx.recv_timeout(RECEIVE_TIMEOUT) {
                    Ok(log_entry) => log_entry,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                if let Err(error) = app_handle.emit("log", &log_entry) {
                    // 这里不能使用 tracing，否则错误事件会重新进入当前通道。
                    eprintln!("向前端推送日志失败: {error}");
                    break;
                }
            }
        })
        .expect("启动日志前端转发线程失败")
}

fn spawn_scan_result_loop(
    rx: mpsc::Receiver<ScanResult>,
    stop: Arc<AtomicBool>,
    app_handle: AppHandle,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("oea-result".to_string())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let result = match rx.recv_timeout(RECEIVE_TIMEOUT) {
                    Ok(result) => result,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                if let Err(error) = app_handle.emit("scan-result", &result) {
                    error!("向前端推送扫描结果失败: {error}");
                }
            }
        })
        .expect("启动扫描结果前端转发线程失败")
}

fn join_thread(name: &str, thread: JoinHandle<()>) {
    if thread.join().is_err() {
        error!(thread = name, "应用常驻后台线程发生 panic");
    }
}
