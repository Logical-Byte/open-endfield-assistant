//! 应用控制器（Tauri 托管状态）。
//!
//! 职责边界：
//! - **应用编排**：根据任务种类创建真实游戏工作者。
//!
//! 自动化任务的状态机与执行线程由 [`automation::Runtime`] 拥有。

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::{
    automation::{self, archive_scan},
    data::{AppData, archive},
    navigation::Navigator,
    settings, vision,
};

/// 应用控制器（Tauri 托管状态）。
pub struct Controller {
    /// 用户设置存储
    settings_store: Arc<settings::SettingsStore>,
    /// 共享 OCR 引擎（跨会话复用模型）
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    /// 导航器（本游戏全部场景，跨线程共享只读）
    navigator: Arc<Navigator>,
    /// 全局唯一自动化任务运行时
    automation_runtime: Arc<automation::Runtime>,
    /// 各领域的只读运行时数据，启动时统一加载
    app_data: Arc<AppData>,
}

impl Controller {
    /// 创建控制器。
    pub(crate) fn new(
        settings_store: Arc<settings::SettingsStore>,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
        navigator: Arc<Navigator>,
        automation_runtime: Arc<automation::Runtime>,
        app_data: AppData,
    ) -> Self {
        Self {
            settings_store,
            ocr,
            navigator,
            automation_runtime,
            app_data: Arc::new(app_data),
        }
    }

    pub fn settings_store(&self) -> &settings::SettingsStore {
        &self.settings_store
    }

    /// 获取当前用户设置的独立快照，供无需持锁的异步流程使用。
    pub fn settings_snapshot(&self) -> settings::OeaSettings {
        self.settings_store.snapshot()
    }

    /// 读取当前自动化状态，空闲状态包含最近一次运行的结束信息。
    pub fn automation_status(&self) -> automation::Status {
        self.automation_runtime.status()
    }

    /// 前端需要的有序精简档案目录。
    pub fn archive_catalog(&self) -> &archive::Catalog {
        self.app_data.archives().catalog()
    }

    // ========== 启动 / 停止 / 退出 ==========

    /// 启动指定种类的自动化任务。
    pub(crate) fn start_automation(&self, request: automation::StartRequest) {
        match request {
            automation::StartRequest::ArchiveScan { worker_type } => {
                self.automation_runtime
                    .start(automation::TaskKind::ArchiveScan, || match worker_type {
                        archive_scan::WorkerType::Production => {
                            Box::new(self.archive_scan_worker())
                        }
                        archive_scan::WorkerType::Simulation => {
                            Box::new(archive_scan::SimulatedArchiveScanWorker::new(Arc::clone(
                                &self.app_data,
                            )))
                        }
                    })
            }
        }
    }

    /// 请求停止当前自动化任务（原子置位，由任务内部轮询实现优雅停止）。
    pub fn stop_automation(&self) {
        self.automation_runtime.stop();
    }

    /// 档案扫描专属快捷入口，供托盘和引号热键维持现有切换行为。
    pub fn toggle_archive_scan(&self) {
        if self.automation_status().is_active() {
            self.stop_automation();
        } else {
            self.start_automation(automation::StartRequest::ArchiveScan {
                worker_type: archive_scan::WorkerType::Production,
            });
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

    fn archive_scan_worker(&self) -> archive_scan::ArchiveScanWorker {
        archive_scan::ArchiveScanWorker::new(
            self.settings_store.snapshot(),
            Arc::clone(&self.ocr),
            Arc::clone(&self.navigator),
            Arc::clone(&self.app_data),
        )
    }
}
