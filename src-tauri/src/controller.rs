//! 扫描业务控制器（Tauri 托管状态）。
//!
//! 职责边界：
//! - **应用编排**：为扫描任务创建运行上下文；
//!
//! 扫描任务的状态机与执行线程由 [`ScanRuntime`] 拥有。

use std::sync::{Arc, Mutex, mpsc};

use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::{
    automation::scan_runtime::{ScanRunContext, ScanRuntime},
    config::{ConfigStore, OeaConfig},
    data::{AppData, ArchiveContract, PrtsData},
    ocr::OcrEngine,
    scene::SceneManager,
    task::archive_scan::{ScanReporter, ScanResult},
};

/// 推送给前端的应用状态。
pub use crate::automation::scan_runtime::AppStatus;

/// 扫描业务控制器（Tauri 托管状态）。
pub struct Controller {
    /// 应用配置存储
    config_store: Arc<ConfigStore>,
    /// 共享 OCR 引擎（跨会话复用模型）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 场景管理器（本游戏全部场景，跨线程共享只读）
    scenes: Arc<SceneManager>,
    /// 扫描档案库任务运行时
    scan_runtime: Arc<ScanRuntime>,
    /// 扫描结果通道发送端（`Mutex` 同理：`Sender` 非 Sync）
    scan_tx: Mutex<mpsc::Sender<ScanResult>>,
    /// 静态数据（prts.json / 档案获取契约 / 纠错索引，启动时统一加载）
    app_data: Arc<AppData>,
}

impl Controller {
    /// 创建控制器。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config_store: Arc<ConfigStore>,
        ocr: Arc<Mutex<OcrEngine>>,
        scenes: Arc<SceneManager>,
        scan_runtime: Arc<ScanRuntime>,
        scan_tx: mpsc::Sender<ScanResult>,
        app_data: AppData,
    ) -> Self {
        Self {
            config_store,
            ocr,
            scenes,
            scan_runtime,
            scan_tx: Mutex::new(scan_tx),
            app_data: Arc::new(app_data),
        }
    }

    pub fn config_store(&self) -> &ConfigStore {
        &self.config_store
    }

    /// 获取当前应用配置的独立快照，供无需持锁的异步流程使用。
    pub fn oea_config_snapshot(&self) -> OeaConfig {
        self.config_store.snapshot()
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
    pub fn start_scan(&self, app_handle: &AppHandle) {
        self.scan_runtime.start(|| self.scan_context(app_handle));
    }

    /// 请求停止扫描档案库任务（原子置位，由任务内部轮询实现优雅停止）。
    pub fn stop_scan(&self) {
        self.scan_runtime.stop();
    }

    pub fn toggle_scan(&self, app_handle: &AppHandle) {
        if self.get_status().running {
            self.stop_scan();
        } else {
            self.start_scan(app_handle);
        }
    }

    /// 退出程序：请求停止后退出 Tauri 应用。
    pub fn quit(&self, app_handle: &AppHandle) {
        if app_handle
            .state::<crate::update::UpdateManager>()
            .is_installing()
        {
            warn!("正在安装更新，拒绝退出");
            return;
        }
        self.scan_runtime.request_stop_for_shutdown();
        info!("收到退出请求，正在退出程序...");
        app_handle.exit(0);
    }

    fn scan_context(&self, app_handle: &AppHandle) -> ScanRunContext {
        ScanRunContext::new(
            self.config_store.snapshot(),
            Arc::clone(&self.ocr),
            Arc::clone(&self.scenes),
            Arc::clone(&self.app_data),
            self.reporter(),
            app_handle.clone(),
        )
    }
}
