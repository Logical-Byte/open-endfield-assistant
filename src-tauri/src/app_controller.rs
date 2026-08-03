//! Tauri 应用控制器。
//!
//! 持有 [`crate::app::App`]、热键监听器与运行/停止标志，作为 Tauri 托管状态：
//! - 前端命令（`invoke`）驱动启动 / 停止 / 单次扫描 / 退出；
//! - 后台线程轮询全局热键事件并执行相同动作；
//! - 通过 Tauri 事件向前端推送状态变更。

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

use crate::{
    app::App,
    hotkey::{HotkeyEvent, HotkeyListener},
    scan_result::ScanResult,
};

/// 推送给前端的应用状态。
#[derive(Debug, Clone, Serialize)]
pub struct AppStatus {
    /// 主任务是否正在运行
    pub running: bool,
}

/// 应用控制器（Tauri 托管状态，以 `Arc` 共享，供命令与后台线程访问）。
pub struct AppController {
    /// 被互斥访问的应用本体（含 Session / TaskRunner）
    app: Mutex<App>,
    /// 全局热键监听器（含 mpsc 接收端，包一层 Mutex 以满足 Sync）
    hotkey: Mutex<HotkeyListener>,
    /// 停止标志（独立于锁：命令 / 热键均可快速请求停止，任务内部轮询）
    stop_flag: Arc<AtomicBool>,
    /// 主任务运行标志（与热键监听器、App 共享，原子读取避免锁竞争）
    running: Arc<AtomicBool>,
    /// Tauri 应用句柄（用于向前端 emit 事件）
    handle: AppHandle,
    /// 日志写入线程守卫（保活，避免 drop 后日志线程关闭）
    _logger_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl AppController {
    /// 创建控制器。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: App,
        hotkey: HotkeyListener,
        stop_flag: Arc<AtomicBool>,
        running: Arc<AtomicBool>,
        handle: AppHandle,
        _logger_guard: tracing_appender::non_blocking::WorkerGuard,
    ) -> Self {
        Self {
            app: Mutex::new(app),
            hotkey: Mutex::new(hotkey),
            stop_flag,
            running,
            handle,
            _logger_guard,
        }
    }

    /// 读取当前状态（只读原子标志，不锁 App）。
    pub fn get_status(&self) -> AppStatus {
        AppStatus {
            running: self.running.load(Ordering::Relaxed),
        }
    }

    /// 启动后台热键轮询线程（每 20ms 非阻塞检查一次）。
    pub fn spawn_hotkey_loop(self: &Arc<Self>) {
        let this = self.clone();
        thread::spawn(move || {
            loop {
                match this.hotkey.lock().unwrap().try_wait_event() {
                    Ok(Some(event)) => this.handle_event(event),
                    Ok(None) => {}
                    Err(e) => {
                        error!("热键监听线程异常退出: {e}");
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
    }

    /// 启动日志转发线程：把 logger 通道里的日志逐行 emit 到前端。
    pub fn spawn_log_loop(rx: mpsc::Receiver<String>, handle: AppHandle) {
        thread::spawn(move || {
            while let Ok(line) = rx.recv() {
                if let Err(e) = handle.emit("log", &line) {
                    eprintln!("向前端推送日志失败: {e}");
                }
            }
        });
    }

    /// 启动扫描结果转发线程：把扫描结果通道里的结果逐个 emit 到前端。
    pub fn spawn_scan_result_loop(rx: mpsc::Receiver<ScanResult>, handle: AppHandle) {
        thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                if let Err(e) = handle.emit("scan-result", &result) {
                    eprintln!("向前端推送扫描结果失败: {e}");
                }
            }
        });
    }

    /// 启动主任务：在独立线程执行扫描，立即返回。
    ///
    /// 用 CAS 原子地占用运行标志，防止热键 / 命令并发重复启动。
    pub fn start_scan(self: &Arc<Self>) {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("主任务正在运行中，忽略重复的启动请求");
            return;
        }
        info!("收到启动主任务请求");
        // 立即向前端推送"运行中"状态（快捷键 / 命令启动都要让界面感知）
        self.emit_status("app-status");
        let this = self.clone();
        thread::spawn(move || {
            let result = {
                let mut app = this.app.lock().unwrap();
                app.start_scan()
            };
            // 无论成功 / 失败 / 被停止，都复位运行标志（会话构建失败时也复位）
            this.running.store(false, Ordering::Relaxed);
            if let Err(e) = result {
                error!("主任务执行失败: {e:#}");
            }
            // 扫描结束，向前端推送"空闲"状态
            this.emit_status("app-status");
        });
    }

    /// 请求停止主任务（设置停止标志，由任务内部轮询实现优雅停止）。
    pub fn stop_scan(&self) {
        if !self.running.load(Ordering::Relaxed) {
            warn!("主任务未在运行，忽略停止请求");
            return;
        }
        self.stop_flag.store(true, Ordering::Relaxed);
        info!("收到停止请求，正在停止主任务...");
    }

    /// 单次扫描当前档案详情。
    pub fn scan_single(&self) {
        if self.running.load(Ordering::Relaxed) {
            warn!("主任务正在运行中，忽略单次扫描请求");
            return;
        }
        let mut app = self.app.lock().unwrap();
        if let Err(e) = app.scan_single() {
            error!("单次扫描失败: {e:#}");
        }
    }

    /// 退出程序：请求停止后退出 Tauri 应用。
    pub fn quit(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        info!("收到退出请求，正在退出程序...");
        self.handle.exit(0);
    }

    /// 处理一个热键事件（空闲时的热键事件；运行中的停止已由热键线程直接处理）。
    fn handle_event(self: &Arc<Self>, event: HotkeyEvent) {
        match event {
            // 空闲时的引号键 → 启动主任务
            HotkeyEvent::ToggleMainTask => self.start_scan(),
            HotkeyEvent::ScanSingleArchive => self.scan_single(),
            HotkeyEvent::ExitProgram => self.quit(),
        }
    }

    /// 向前端推送当前状态。
    fn emit_status(&self, event_name: &str) {
        let status = self.get_status();
        if let Err(e) = self.handle.emit(event_name, &status) {
            error!("向前端推送状态 {event_name:?} 失败: {e}");
        }
    }
}
