//! 应用控制器模块。
//!
//! 类比 MaaFramework 中负责顶层调度与状态管理的组件：持有 [`Session`]、
//! 场景管理器、任务运行器和热键监听器，运行主事件循环——空闲时阻塞等待
//! 热键事件，并分发到对应的处理函数。
//!
//! 脚本启动后**不会自动运行任何任务**，而是进入空闲等待状态，由热键驱动：
//! - 分号键 `;` → 单次扫描当前档案详情（仅截屏识别）
//! - 引号键 `'` → 启动 / 停止档案库主任务
//! - Alt+Delete → 退出程序

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use tracing::{error, info, warn};

use crate::{
    scene::scene_manager::SceneManager,
    session::Session,
    task::{TaskRunner, TaskStopped},
    tasks::archive_scan::{ArchiveScanTask, scan_single_archive_detail},
    window,
};

/// 应用控制器。
///
/// 由 [`crate::app_controller::AppController`] 持有并以互斥方式驱动；
/// 阻塞逻辑（扫描）在独立线程运行，不在 Tauri 主线程执行。
pub struct App {
    /// 脚本会话（聚合截图、输入、OCR、模板匹配等能力）
    session: Session,
    /// 任务运行器（管理场景检测、导航与任务执行）
    task_runner: TaskRunner,
    /// 主任务是否正在运行（与热键监听器共享，原子标志）
    running: Arc<AtomicBool>,
}

impl App {
    /// 创建应用控制器。
    ///
    /// # 参数
    /// - `session`: 已初始化的会话
    /// - `scene_manager`: 已注册所有场景并构建导航图的场景管理器
    /// - `running`: 主任务运行标志（与热键监听器共享的 `Arc<AtomicBool>`）
    pub fn new(session: Session, scene_manager: SceneManager, running: Arc<AtomicBool>) -> Self {
        Self {
            session,
            task_runner: TaskRunner::new(scene_manager),
            running,
        }
    }

    /// 主任务是否正在运行。
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// 启动主任务（扫描档案库）。
    ///
    /// 阻塞执行整个扫描过程；由调用方（[`crate::app_controller::AppController`]）
    /// 在独立线程运行。期间通过停止标志（Session 轮询）支持优雅停止。
    ///
    /// 注意：调用方已通过 CAS 原子占用运行标志，保证此方法不会被并发重复调用，
    /// 因此这里不做 `is_running` 防御（否则会与 CAS 设置的运行标志冲突导致跳过执行）。
    pub fn start_scan(&mut self) -> Result<()> {
        info!("========== 启动主任务：扫描档案库 ==========");

        // 主任务需要点击游戏窗口，先确保窗口在前台；失败不阻断，仅警告
        if let Err(e) = window::ensure_foreground_and_topmost(self.session.hwnd) {
            warn!("无法将游戏窗口置于前台: {e:#}，继续尝试执行任务");
        }

        // 清除上一次可能残留的停止标志，并标记运行状态（热键/命令据此响应停止）
        self.session.reset_stop();
        self.running.store(true, Ordering::Relaxed);

        // 执行主任务（阻塞，期间任务内部轮询停止标志）
        let result = self
            .task_runner
            .run_task(&ArchiveScanTask, &mut self.session);

        // 无论成功 / 被停止 / 失败，都要复位运行状态
        self.running.store(false, Ordering::Relaxed);

        match result {
            Ok(_) => info!("========== 主任务执行完毕 =========="),
            Err(e) if e.downcast_ref::<TaskStopped>().is_some() => {
                info!("主任务已被用户停止");
            }
            Err(e) => error!("主任务执行失败: {e:#}"),
        }
        Ok(())
    }

    /// 单次扫描当前档案详情（分号键 / 前端「单次扫描」）。
    ///
    /// 仅截屏识别档案标题，不做任何鼠标键盘输入操作。
    pub fn scan_single(&mut self) -> Result<()> {
        // 防御：主任务运行中忽略（热键线程也会过滤，这里双重保险）
        if self.is_running() {
            warn!("主任务正在运行中，忽略单次扫描请求");
            return Ok(());
        }

        // 清除上一次可能残留的停止标志，避免单次扫描被误判为停止
        self.session.reset_stop();

        scan_single_archive_detail(&mut self.session, self.task_runner.scene_manager())?;
        Ok(())
    }
}
