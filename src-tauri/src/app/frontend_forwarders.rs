//! 后端通道到 Tauri 前端事件的转发线程。

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tauri::{AppHandle, Emitter};
use tracing::error;

use crate::{logger::LogEntry, task::archive_scan::ScanResult};

const RECEIVE_TIMEOUT: Duration = Duration::from_millis(100);

/// 启动日志前端转发线程。
pub(super) fn spawn_log_forwarder(
    rx: Receiver<LogEntry>,
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

/// 启动扫描结果前端转发线程。
pub(super) fn spawn_scan_result_forwarder(
    rx: Receiver<ScanResult>,
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
