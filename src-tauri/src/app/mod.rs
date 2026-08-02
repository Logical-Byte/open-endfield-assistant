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

use anyhow::Result;
use tracing::{error, info, warn};

use crate::{
    hotkey::{HotkeyEvent, HotkeyListener},
    scene::scene_manager::SceneManager,
    session::Session,
    task::{TaskRunner, TaskStopped},
    tasks::archive_scan::{ArchiveScanTask, scan_single_archive_detail},
    window,
};

/// 应用控制器。
///
/// 生命周期：
/// 1. [`App::new`] 组装所有组件
/// 2. [`App::run`] 进入主事件循环，由热键驱动，正常情况下不会退出
pub struct App {
    /// 脚本会话（聚合截图、输入、OCR、模板匹配等能力）
    session: Session,
    /// 任务运行器（管理场景检测、导航与任务执行）
    task_runner: TaskRunner,
    /// 热键监听器
    hotkey: HotkeyListener,
}

impl App {
    /// 创建应用控制器。
    ///
    /// # 参数
    /// - `session`: 已初始化的会话
    /// - `scene_manager`: 已注册所有场景并构建导航图的场景管理器
    /// - `hotkey`: 已注册热键的监听器
    pub fn new(session: Session, scene_manager: SceneManager, hotkey: HotkeyListener) -> Self {
        Self {
            session,
            task_runner: TaskRunner::new(scene_manager),
            hotkey,
        }
    }

    /// 运行主事件循环。
    ///
    /// 空闲时阻塞等待热键事件并分发处理。单个事件处理失败只记录错误日志，
    /// 不会中断整个脚本；收到退出事件（Alt+Delete）或热键监听线程异常退出时返回。
    pub fn run(&mut self) -> Result<()> {
        info!(
            "脚本已就绪：按「;」扫描当前档案详情，按「'」启动/停止档案库扫描，按「Alt+Delete」退出程序"
        );

        loop {
            let event = self.hotkey.wait_event()?;
            match event {
                // Alt+Delete：退出程序（主任务运行中的停止请求已由热键线程处理，
                // 任务结束后这里才会收到退出事件）
                HotkeyEvent::ExitProgram => {
                    info!("收到退出请求，正在退出程序...");
                    return Ok(());
                }
                event => {
                    let result = match event {
                        HotkeyEvent::ToggleMainTask => self.toggle_main_task(),
                        HotkeyEvent::ScanSingleArchive => self.scan_single(),
                        HotkeyEvent::ExitProgram => unreachable!(),
                    };
                    if let Err(e) = result {
                        error!("处理热键事件 {:?} 失败: {e:#}", event);
                    }
                }
            }
        }
    }

    /// 切换主任务（扫描档案库）运行状态。
    ///
    /// 空闲时按引号键启动；主任务运行中的停止请求由热键线程直接设置
    /// 停止标志处理，因此这里只会收到"启动"分支。
    fn toggle_main_task(&mut self) -> Result<()> {
        // 防御：如果已经在运行（理论上不会发生），忽略。
        if self.hotkey.is_main_running() {
            warn!("主任务正在运行中，忽略重复的启动请求");
            return Ok(());
        }

        info!("========== 启动主任务：扫描档案库 ==========");

        // 主任务需要点击游戏窗口，先确保窗口在前台；失败不阻断，仅警告
        if let Err(e) = window::ensure_foreground_and_topmost(self.session.hwnd) {
            warn!("无法将游戏窗口置于前台: {e:#}，继续尝试执行任务");
        }

        // 重置停止标志并标记运行状态（热键线程据此响应停止请求）
        self.session.reset_stop();
        self.hotkey.set_main_running(true);

        // 执行主任务
        let result = self
            .task_runner
            .run_task(&ArchiveScanTask, &mut self.session);

        // 无论成功 / 被停止 / 失败，都要复位运行状态
        self.hotkey.set_main_running(false);

        match result {
            Ok(_) => info!("========== 主任务执行完毕 =========="),
            Err(e) if e.downcast_ref::<TaskStopped>().is_some() => {
                info!("主任务已被用户停止");
            }
            Err(e) => error!("主任务执行失败: {e:#}"),
        }
        Ok(())
    }

    /// 单次扫描当前档案详情（分号键）。
    ///
    /// 仅截屏识别档案标题，不做任何鼠标键盘输入操作。
    fn scan_single(&mut self) -> Result<()> {
        // 防御：主任务运行中忽略（热键线程也会过滤，这里双重保险）
        if self.hotkey.is_main_running() {
            warn!("主任务正在运行中，忽略单次扫描请求");
            return Ok(());
        }

        // 清除上一次可能残留的停止标志，避免单次扫描被误判为停止
        self.session.reset_stop();

        scan_single_archive_detail(&mut self.session, self.task_runner.scene_manager())?;
        Ok(())
    }
}
