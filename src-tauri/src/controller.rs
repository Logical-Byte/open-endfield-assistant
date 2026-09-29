//! 应用控制器（Tauri 托管状态）。
//!
//! 职责边界：
//! - **应用编排**：根据任务种类创建真实游戏工作者；
//!
//! 自动化任务的状态机与执行线程由 [`Runtime`] 拥有。

use std::sync::{Arc, Mutex, mpsc};

use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::{
    automation::{
        self,
        archive_scan::{ScanResult, worker::ArchiveScanWorker},
    },
    config::{ConfigStore, OeaConfig},
    data::{AppData, ArchiveContract, PrtsData},
    navigation::Navigator,
    ocr::OcrEngine,
};

/// 应用控制器（Tauri 托管状态）。
pub struct Controller {
    /// 应用配置存储
    config_store: Arc<ConfigStore>,
    /// 共享 OCR 引擎（跨会话复用模型）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 导航器（本游戏全部场景，跨线程共享只读）
    navigator: Arc<Navigator>,
    /// 全局唯一自动化任务运行时
    automation_runtime: Arc<automation::Runtime>,
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
        navigator: Arc<Navigator>,
        automation_runtime: Arc<automation::Runtime>,
        scan_tx: mpsc::Sender<ScanResult>,
        app_data: AppData,
    ) -> Self {
        Self {
            config_store,
            ocr,
            navigator,
            automation_runtime,
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

    /// 读取当前自动化状态；任务终态由一次性事件单独推送。
    pub fn automation_status(&self) -> automation::Status {
        self.automation_runtime.status()
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

    /// 启动指定种类的自动化任务。
    pub fn start_automation(&self, app_handle: &AppHandle, task_kind: automation::TaskKind) {
        match task_kind {
            automation::TaskKind::ArchiveScan => {
                self.automation_runtime.start(app_handle, task_kind, || {
                    Box::new(self.archive_scan_worker())
                })
            }
        }
    }

    /// 请求停止当前自动化任务（原子置位，由任务内部轮询实现优雅停止）。
    pub fn stop_automation(&self) {
        self.automation_runtime.stop();
    }

    /// 档案扫描专属快捷入口，供托盘和引号热键维持现有切换行为。
    pub fn toggle_archive_scan(&self, app_handle: &AppHandle) {
        if self.automation_status().is_active() {
            self.stop_automation();
        } else {
            self.start_automation(app_handle, automation::TaskKind::ArchiveScan);
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
        self.automation_runtime.request_stop_for_shutdown();
        info!("收到退出请求，正在退出程序...");
        app_handle.exit(0);
    }

    fn archive_scan_worker(&self) -> ArchiveScanWorker {
        ArchiveScanWorker::new(
            self.config_store.snapshot(),
            Arc::clone(&self.ocr),
            Arc::clone(&self.navigator),
            Arc::clone(&self.app_data),
            self.scan_tx.lock().unwrap().clone(),
        )
    }
}
