//! 应用控制器（Tauri 托管状态）。
//!
//! 职责边界：
//! - **应用编排**：为扫描任务创建运行上下文；
//! - **线程编排**：热键消费线程、日志 / 结果转发线程；
//! - **热键动作分发**（应用层）：键位常量（[`TOGGLE_MAIN_TASK_HOTKEY`] / [`EXIT_HOTKEY`]）+ 前台窗口过滤 + [`Controller::spawn_hotkey_loop`]；
//!
//! 扫描任务的状态机与执行线程由 [`ScanRuntime`] 拥有。

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info, warn};

use crate::{
    app_paths::AppPaths,
    config::OeaConfig,
    data::{AppData, ArchiveContract, PrtsData},
    logger::LogEntry,
    ocr::OcrEngine,
    scan_runtime::{ScanRunContext, ScanRuntime},
    scene::SceneManager,
    task::archive_scan::{ScanReporter, ScanResult},
    windows_ops,
};

/// 推送给前端的应用状态。
pub use crate::scan_runtime::AppStatus;

/// 引号 `'` → 切换扫描档案库任务
pub const TOGGLE_MAIN_TASK_HOTKEY: windows_ops::hotkey::KeyEvent = windows_ops::hotkey::KeyEvent {
    vk: windows_ops::hotkey::OEM_7_KEY,
    down: true,
    modifiers: 0,
};
/// Alt+Delete → 退出
pub const EXIT_HOTKEY: windows_ops::hotkey::KeyEvent = windows_ops::hotkey::KeyEvent {
    vk: windows_ops::hotkey::DELETE_KEY,
    down: true,
    modifiers: windows_ops::hotkey::ALT_MODIFIER,
};

/// 应用控制器（Tauri 托管状态，以 `Arc` 共享）。
pub struct Controller {
    /// 应用根目录
    app_path: AppPaths,
    /// 应用配置
    oea_config: Arc<Mutex<OeaConfig>>,
    /// 共享 OCR 引擎（跨会话复用模型）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 场景管理器（本游戏全部场景，跨线程共享只读）
    scenes: Arc<SceneManager>,
    /// 扫描档案库任务运行时
    scan_runtime: Arc<ScanRuntime>,
    /// 扫描结果通道发送端（`Mutex` 同理：`Sender` 非 Sync）
    scan_tx: Mutex<mpsc::Sender<ScanResult>>,
    /// 前台窗口守卫（应用层过滤：分号/引号仅在前台为 OEA 或终末地时响应）
    foreground: windows_ops::window::ForegroundGuard,
    /// Tauri 应用句柄（创建扫描运行上下文）
    handle: AppHandle,
    /// 静态数据（prts.json / 档案获取契约 / 纠错索引，启动时统一加载）
    app_data: Arc<AppData>,
    /// 日志写入线程守卫（保活）
    _logger_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl Controller {
    /// 创建控制器。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        app_path: AppPaths,
        oea_config: Arc<Mutex<OeaConfig>>,
        ocr: Arc<Mutex<OcrEngine>>,
        scenes: Arc<SceneManager>,
        scan_runtime: Arc<ScanRuntime>,
        scan_tx: mpsc::Sender<ScanResult>,
        foreground: windows_ops::window::ForegroundGuard,
        handle: AppHandle,
        app_data: AppData,
        _logger_guard: tracing_appender::non_blocking::WorkerGuard,
    ) -> Self {
        Self {
            app_path,
            oea_config,
            ocr,
            scenes,
            scan_runtime,
            scan_tx: Mutex::new(scan_tx),
            foreground,
            handle,
            app_data: Arc::new(app_data),
            _logger_guard,
        }
    }

    pub fn app_path(&self) -> &AppPaths {
        &self.app_path
    }

    pub fn oea_config(&self) -> &Arc<Mutex<OeaConfig>> {
        &self.oea_config
    }

    /// 读取当前状态（只读原子标志；失败原因不存储，由结束事件一次性推送）。
    pub fn get_status(&self) -> AppStatus {
        self.scan_runtime.status()
    }

    /// 创建扫描结果上报器（每次游戏操作一个，只负责转发结果）。
    fn reporter(&self) -> ScanReporter {
        ScanReporter::new(self.scan_tx.lock().unwrap().clone())
    }

    /// 返回 prts.json 完整数据（供前端查询分类中文名 / 自动补全候选）。
    pub fn prts_data(&self) -> &PrtsData {
        self.app_data.prts()
    }

    /// 返回档案获取契约完整数据（供前端按档案 id 查询获取方式）。
    pub fn archive_contract_data(&self) -> &ArchiveContract {
        self.app_data.archive_contract()
    }

    // ========== 启动 / 停止 / 退出 ==========

    /// 启动扫描档案库任务：CAS 占用运行标志 → 推送状态 → 后台线程执行。
    pub fn start_scan(self: &Arc<Self>) {
        self.scan_runtime.start(|| self.scan_context());
    }

    /// 请求停止扫描档案库任务（原子置位，由任务内部轮询实现优雅停止）。
    pub fn stop_scan(&self) {
        self.scan_runtime.stop();
    }

    pub fn toggle_scan(self: &Arc<Self>) {
        if self.get_status().running {
            self.stop_scan();
        } else {
            self.start_scan();
        }
    }

    /// 退出程序：请求停止后退出 Tauri 应用。
    pub fn quit(&self) {
        if crate::update::is_installing() {
            warn!("正在安装更新，拒绝退出");
            return;
        }
        self.scan_runtime.request_stop_for_shutdown();
        info!("收到退出请求，正在退出程序...");
        self.handle.exit(0);
    }

    // ========== 后台线程 ==========

    /// 启动热键消费线程（应用层：前台窗口过滤 + 动作分发）。
    pub fn spawn_hotkey_loop(self: &Arc<Self>, rx: mpsc::Receiver<windows_ops::hotkey::KeyEvent>) {
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

    fn scan_context(&self) -> ScanRunContext {
        ScanRunContext::new(
            self.app_path.clone(),
            Arc::clone(&self.oea_config),
            Arc::clone(&self.ocr),
            Arc::clone(&self.scenes),
            Arc::clone(&self.app_data),
            self.reporter(),
            self.handle.clone(),
        )
    }
}
