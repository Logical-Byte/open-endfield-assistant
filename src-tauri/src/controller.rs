//! 应用控制器（Tauri 托管状态）。
//!
//! 职责边界：
//! - **状态机**：`running`（CAS 防重入启动）与 `stop`（秒停）两个原子标志的**唯一归属**；
//! - **线程编排**：扫描档案库任务线程、热键消费线程、日志 / 结果转发线程；
//! - **热键动作分发**（应用层）：键位常量（[`TOGGLE_MAIN_TASK_HOTKEY`] / [`EXIT_HOTKEY`]）+ 前台窗口过滤 + [`Controller::spawn_hotkey_loop`]；
//!
//! 依赖方向：应用层 → 领域层（connect/session/scene/task）→ 基础设施层。

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info, warn};
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, VK_DELETE, VK_OEM_7};

use crate::{
    app_paths::AppPaths,
    config::OeaConfig,
    connect::connect_to_game,
    data::AppData,
    hotkey::KeyEvent,
    logger::LogEntry,
    ocr::OcrEngine,
    scene::SceneManager,
    task::{TaskStopped, run_task},
    tasks::archive_scan::{ArchiveScanTask, ScanReporter, ScanResult},
    types::{ArchiveAcquisitionContract, PrtsData},
    window::{self, ForegroundGuard},
};

/// 推送给前端的应用状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    /// 扫描档案库任务是否正在运行
    pub running: bool,
}

/// 引号 `'` → 切换扫描档案库任务
pub const TOGGLE_MAIN_TASK_HOTKEY: KeyEvent = KeyEvent {
    vk: VK_OEM_7.0 as u32,
    down: true,
    modifiers: 0,
};
/// Alt+Delete → 退出
pub const EXIT_HOTKEY: KeyEvent = KeyEvent {
    vk: VK_DELETE.0 as u32,
    down: true,
    modifiers: MOD_ALT.0,
};

/// 应用控制器（Tauri 托管状态，以 `Arc` 共享）。
pub struct Controller {
    /// 应用根目录
    app_path: AppPaths,
    /// 应用配置
    oea_config: Mutex<OeaConfig>,
    /// 共享 OCR 引擎（跨会话复用模型）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 场景管理器（本游戏全部场景，跨线程共享只读）
    scenes: Arc<SceneManager>,
    /// 停止标志（独立于锁：命令 / 热键均可快速请求停止，任务内部轮询）
    stop: Arc<AtomicBool>,
    /// 扫描档案库任务运行标志（CAS 占用防重入）
    running: Arc<AtomicBool>,
    /// 扫描结果通道发送端（`Mutex` 同理：`Sender` 非 Sync）
    scan_tx: Mutex<mpsc::Sender<ScanResult>>,
    /// 全局扫描序号（跨扫描档案库任务连续递增）
    scan_index: Arc<AtomicU32>,
    /// 前台窗口守卫（应用层过滤：分号/引号仅在前台为 OEA 或终末地时响应）
    foreground: ForegroundGuard,
    /// Tauri 应用句柄（向前端 emit 事件）
    handle: AppHandle,
    /// 静态数据（prts.json / 档案获取契约 / 纠错索引，启动时统一加载）
    app_data: AppData,
    /// 日志写入线程守卫（保活）
    _logger_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl Controller {
    /// 创建控制器。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app_path: AppPaths,
        oea_config: Mutex<OeaConfig>,
        ocr: Arc<Mutex<OcrEngine>>,
        scenes: Arc<SceneManager>,
        stop: Arc<AtomicBool>,
        running: Arc<AtomicBool>,
        scan_tx: mpsc::Sender<ScanResult>,
        scan_index: Arc<AtomicU32>,
        foreground: ForegroundGuard,
        handle: AppHandle,
        app_data: AppData,
        _logger_guard: tracing_appender::non_blocking::WorkerGuard,
    ) -> Self {
        Self {
            app_path,
            oea_config,
            ocr,
            scenes,
            stop,
            running,
            scan_tx: Mutex::new(scan_tx),
            scan_index,
            foreground,
            handle,
            app_data,
            _logger_guard,
        }
    }

    pub fn app_path(&self) -> &AppPaths {
        &self.app_path
    }

    pub fn oea_config(&self) -> &Mutex<OeaConfig> {
        &self.oea_config
    }

    /// 读取当前状态（只读原子标志，不锁任何互斥量）。
    pub fn get_status(&self) -> AppStatus {
        AppStatus {
            running: self.running.load(Ordering::Relaxed),
        }
    }

    /// 创建扫描结果上报器（每次游戏操作一个，序号全局连续）。
    fn reporter(&self) -> ScanReporter {
        ScanReporter::new(
            self.scan_tx.lock().unwrap().clone(),
            Arc::clone(&self.scan_index),
        )
    }

    /// 返回 prts.json 完整数据（供前端查询分类中文名 / 自动补全候选）。
    pub fn prts_data(&self) -> Arc<PrtsData> {
        self.app_data.prts()
    }

    /// 返回档案获取契约完整数据（供前端按档案 id 查询获取方式）。
    pub fn archive_acquisition_contract_data(&self) -> Arc<ArchiveAcquisitionContract> {
        self.app_data.archive_acquisition_contract()
    }

    // ========== 启动 / 停止 / 退出 ==========

    /// 启动扫描档案库任务：CAS 占用运行标志 → 推送状态 → 后台线程执行。
    pub fn start_scan(self: &Arc<Self>) {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("扫描档案库任务正在运行中，忽略重复的启动请求");
            return;
        }
        info!("收到启动扫描档案库任务请求");
        self.emit_status();

        let this = Arc::clone(self);
        thread::Builder::new()
            .name("oea-scan".to_string())
            .spawn(move || {
                // 任务开始时才连接游戏（游戏未打开则报错并复位）
                let mut session = match connect_to_game(
                    &this.ocr,
                    &this.app_path.templates_dir(),
                    this.stop.clone(),
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("连接游戏失败: {e:#}");
                        this.finish_scan();
                        return;
                    }
                };

                // 扫描档案库任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）
                if let Err(e) = window::ensure_foreground_and_topmost(session.hwnd) {
                    warn!("无法将游戏窗口置于前台: {e:#}，继续尝试执行任务");
                }

                // 清除上一次可能残留的停止信号
                session.reset_stop();

                // 执行扫描档案库任务（阻塞，期间任务内部轮询停止标志）
                let task = ArchiveScanTask::new(this.reporter(), this.app_data.correction());
                let result = run_task(&task, &mut session, &this.scenes);

                // 区分"被停止"与"出错"
                match result {
                    Ok(_) => info!("========== 扫描档案库任务执行完毕 =========="),
                    Err(e) if e.downcast_ref::<TaskStopped>().is_some() => {
                        info!("扫描档案库任务已被用户停止");
                    }
                    Err(e) => error!("扫描档案库任务执行失败: {e:#}"),
                }

                this.finish_scan();
            })
            .expect("启动扫描档案库任务线程失败");
    }

    /// 复位运行标志并推送"空闲"状态。
    fn finish_scan(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.emit_status();
    }

    /// 请求停止扫描档案库任务（原子置位，由任务内部轮询实现优雅停止）。
    pub fn stop_scan(&self) {
        if !self.running.load(Ordering::Relaxed) {
            warn!("扫描档案库任务未在运行，忽略停止请求");
            return;
        }
        self.stop.store(true, Ordering::Relaxed);
        info!("收到停止请求，正在停止扫描档案库任务...");
    }

    pub fn toggle_scan(self: &Arc<Self>) {
        if self.running.load(Ordering::Relaxed) {
            self.stop_scan();
        } else {
            self.start_scan();
        }
    }

    /// 退出程序：请求停止后退出 Tauri 应用。
    pub fn quit(&self) {
        self.stop.store(true, Ordering::Relaxed);
        info!("收到退出请求，正在退出程序...");
        self.handle.exit(0);
    }

    // ========== 后台线程 ==========

    /// 启动热键消费线程（应用层：前台窗口过滤 + 动作分发）。
    pub fn spawn_hotkey_loop(self: &Arc<Self>, rx: mpsc::Receiver<KeyEvent>) {
        let self_cloned = Arc::clone(self);

        thread::Builder::new()
            .name("oea-hotkey".to_string())
            .spawn(move || {
                while let Ok(key_event) = rx.recv() {
                    if key_event == EXIT_HOTKEY {
                        self_cloned.quit();
                    } else if key_event == TOGGLE_MAIN_TASK_HOTKEY {
                        if self_cloned.foreground.is_foreground_eligible() {
                            self_cloned.toggle_scan();
                        } else {
                            debug!("前台窗口不是终末地或者 OEA，忽略热键");
                        }
                    }
                }
            })
            .expect("启动热键消费线程失败");
    }

    /// 启动日志转发线程：把 logger 通道里的日志逐条 emit 到前端。
    pub fn spawn_log_loop(rx: mpsc::Receiver<LogEntry>, handle: AppHandle) {
        thread::Builder::new()
            .name("oea-log".to_string())
            .spawn(move || {
                while let Ok(log_entry) = rx.recv() {
                    if let Err(e) = handle.emit("log", &log_entry) {
                        error!("向前端推送日志失败: {e}");
                    }
                }
            })
            .expect("启动日志转发线程失败");
    }

    /// 启动扫描结果转发线程：把结果通道里的结果逐个 emit 到前端。
    pub fn spawn_scan_result_loop(rx: mpsc::Receiver<ScanResult>, handle: AppHandle) {
        thread::Builder::new()
            .name("oea-result".to_string())
            .spawn(move || {
                while let Ok(result) = rx.recv() {
                    if let Err(e) = handle.emit("scan-result", &result) {
                        error!("向前端推送扫描结果失败: {e}");
                    }
                }
            })
            .expect("启动扫描结果转发线程失败");
    }

    /// 向前端推送当前状态。
    fn emit_status(&self) {
        let status = self.get_status();
        if let Err(e) = self.handle.emit("app-status", &status) {
            error!("向前端推送状态失败: {e}");
        }
    }
}
