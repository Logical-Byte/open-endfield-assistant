//! 应用控制器（Tauri 托管状态）。
//!
//! 职责边界：
//! - **状态机**：`running`（CAS 防重入启动）与 `stop`（秒停）两个原子标志的**唯一归属**；
//! - **线程编排**：主任务扫描线程、热键轮询线程、日志 / 结果转发线程；
//! - **热键动作分发**（应用层）：键位 → 动作的绑定表 + 前台规则 + [`handle_hotkey`]；
//!
//! 依赖方向：应用层 → 领域层（connect/session/scene/task）→ 基础设施层。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_NOREPEAT, VK_DELETE, VK_OEM_1, VK_OEM_7,
};

use crate::{
    connect::connect_to_game,
    hotkey::{HotkeyBinding, HotkeyEvent},
    ocr::OcrEngine,
    scene::SceneManager,
    task::{TaskStopped, run_task},
    tasks::archive_scan::{ArchiveScanTask, ScanReporter, ScanResult, single_scan},
    window,
};

/// 推送给前端的应用状态。
#[derive(Debug, Clone, Serialize)]
pub struct AppStatus {
    /// 主任务是否正在运行
    pub running: bool,
}

/// 应用层热键动作（键位表见 [`HOTKEY_BINDINGS`]）。
///
/// `as u32` 即热键标签（基础设施层只透传标签，不关心动作语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    /// 切换主任务：空闲=启动，运行中=停止
    ToggleMainTask = 0,
    /// 单次扫描当前档案详情
    ScanSingle = 1,
    /// 退出程序
    Exit = 2,
}

impl HotkeyAction {
    /// 从热键标签还原动作。
    fn from_tag(tag: u32) -> Option<Self> {
        match tag {
            0 => Some(Self::ToggleMainTask),
            1 => Some(Self::ScanSingle),
            2 => Some(Self::Exit),
            _ => None,
        }
    }
}

/// 应用层键位表：虚拟键码 + 修饰符 → 动作。
///
/// - 分号 `;` → 单次扫描
/// - 引号 `'` → 切换主任务
/// - Alt+Delete → 退出
pub const HOTKEY_BINDINGS: &[HotkeyBinding] = &[
    HotkeyBinding {
        vk: VK_OEM_1.0 as u32,
        modifiers: 0,
        tag: HotkeyAction::ScanSingle as u32,
    },
    HotkeyBinding {
        vk: VK_OEM_7.0 as u32,
        modifiers: 0,
        tag: HotkeyAction::ToggleMainTask as u32,
    },
    HotkeyBinding {
        vk: VK_DELETE.0 as u32,
        modifiers: MOD_ALT.0 | MOD_NOREPEAT.0,
        tag: HotkeyAction::Exit as u32,
    },
];

/// 应用控制器（Tauri 托管状态，以 `Arc` 共享）。
pub struct Controller {
    /// 共享 OCR 引擎（跨会话复用模型）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 模板图片根目录
    templates_root: PathBuf,
    /// 场景管理器（本游戏全部场景，跨线程共享只读）
    scenes: Arc<SceneManager>,
    /// 停止标志（独立于锁：命令 / 热键均可快速请求停止，任务内部轮询）
    stop: Arc<AtomicBool>,
    /// 主任务运行标志（CAS 占用防重入）
    running: Arc<AtomicBool>,
    /// 游戏操作串行门：同一时刻只允许一个操作（主任务 / 单次扫描）
    op_lock: Mutex<()>,
    /// 扫描结果通道发送端（`Mutex` 同理：`Sender` 非 Sync）
    scan_tx: Mutex<mpsc::Sender<ScanResult>>,
    /// 全局扫描序号（跨主任务 / 单次扫描连续递增）
    scan_index: Arc<AtomicU32>,
    /// Tauri 应用句柄（向前端 emit 事件）
    handle: AppHandle,
    /// 日志写入线程守卫（保活）
    _logger_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl Controller {
    /// 创建控制器。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ocr: Arc<Mutex<OcrEngine>>,
        templates_root: PathBuf,
        scenes: Arc<SceneManager>,
        stop: Arc<AtomicBool>,
        running: Arc<AtomicBool>,
        scan_tx: mpsc::Sender<ScanResult>,
        scan_index: Arc<AtomicU32>,
        handle: AppHandle,
        _logger_guard: tracing_appender::non_blocking::WorkerGuard,
    ) -> Self {
        Self {
            ocr,
            templates_root,
            scenes,
            stop,
            running,
            op_lock: Mutex::new(()),
            scan_tx: Mutex::new(scan_tx),
            scan_index,
            handle,
            _logger_guard,
        }
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

    // ========== 启动 / 停止 / 单扫 / 退出 ==========

    /// 启动主任务：CAS 占用运行标志 → 推送状态 → 后台线程执行。
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
        self.emit_status();

        let this = Arc::clone(self);
        thread::spawn(move || {
            // 游戏操作串行门：与单次扫描互斥
            let _gate = this.op_lock.lock().unwrap();

            // 任务开始时才连接游戏（游戏未打开则报错并复位）
            let mut session =
                match connect_to_game(&this.ocr, &this.templates_root, this.stop.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("连接游戏失败: {e:#}");
                        this.finish_scan();
                        return;
                    }
                };

            // 主任务需要点击游戏窗口，先确保窗口在前台（失败不阻断）
            if let Err(e) = window::ensure_foreground_and_topmost(session.hwnd) {
                warn!("无法将游戏窗口置于前台: {e:#}，继续尝试执行任务");
            }

            // 清除上一次可能残留的停止信号
            session.reset_stop();

            // 执行主任务（阻塞，期间任务内部轮询停止标志）
            let task = ArchiveScanTask::new(this.reporter());
            let result = run_task(&task, &mut session, &this.scenes);

            // 区分"被停止"与"出错"
            match result {
                Ok(_) => info!("========== 主任务执行完毕 =========="),
                Err(e) if e.downcast_ref::<TaskStopped>().is_some() => {
                    info!("主任务已被用户停止");
                }
                Err(e) => error!("主任务执行失败: {e:#}"),
            }

            this.finish_scan();
        });
    }

    /// 复位运行标志并推送"空闲"状态。
    fn finish_scan(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.emit_status();
    }

    /// 请求停止主任务（原子置位，由任务内部轮询实现优雅停止）。
    pub fn stop_scan(&self) {
        if !self.running.load(Ordering::Relaxed) {
            warn!("主任务未在运行，忽略停止请求");
            return;
        }
        self.stop.store(true, Ordering::Relaxed);
        info!("收到停止请求，正在停止主任务...");
    }

    /// 单次扫描当前档案详情（在调用线程同步执行）。
    pub fn scan_single(&self) {
        if self.running.load(Ordering::Relaxed) {
            warn!("主任务正在运行中，忽略单次扫描请求");
            return;
        }
        let _gate = self.op_lock.lock().unwrap();
        // 二次检查：等待串行门期间主任务可能已启动
        if self.running.load(Ordering::Relaxed) {
            warn!("主任务正在运行中，忽略单次扫描请求");
            return;
        }

        let mut session = match connect_to_game(&self.ocr, &self.templates_root, self.stop.clone())
        {
            Ok(s) => s,
            Err(e) => {
                error!("连接游戏失败: {e:#}");
                return;
            }
        };
        session.reset_stop();

        if let Err(e) = single_scan(&mut session, &self.scenes, &self.reporter()) {
            error!("单次扫描失败: {e:#}");
        }
    }

    /// 退出程序：请求停止后退出 Tauri 应用。
    pub fn quit(&self) {
        self.stop.store(true, Ordering::Relaxed);
        info!("收到退出请求，正在退出程序...");
        self.handle.exit(0);
    }

    // ========== 热键动作分发（应用层） ==========

    /// 处理一个热键动作（运行中 / 空闲分派收敛于此）。
    fn handle_hotkey(self: &Arc<Self>, action: HotkeyAction) {
        match action {
            HotkeyAction::ToggleMainTask => {
                if self.running.load(Ordering::Relaxed) {
                    self.stop.store(true, Ordering::Relaxed);
                    info!("收到停止请求（热键），正在停止主任务...");
                } else {
                    self.start_scan();
                }
            }
            HotkeyAction::ScanSingle => {
                if !self.running.load(Ordering::Relaxed) {
                    self.scan_single();
                }
            }
            HotkeyAction::Exit => {
                if self.running.load(Ordering::Relaxed) {
                    info!("收到退出请求（热键），正在停止主任务...");
                }
                self.quit();
            }
        }
    }

    // ========== 后台线程 ==========

    /// 启动热键消费线程：阻塞接收事件并立即分发（消息驱动，无轮询）。
    pub fn spawn_hotkey_loop(rx: mpsc::Receiver<HotkeyEvent>, this: Arc<Self>) {
        thread::spawn(move || {
            while let Ok(HotkeyEvent { tag }) = rx.recv() {
                match HotkeyAction::from_tag(tag) {
                    Some(action) => this.handle_hotkey(action),
                    None => warn!("未知热键标签: {tag}"),
                }
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

    /// 启动扫描结果转发线程：把结果通道里的结果逐个 emit 到前端。
    pub fn spawn_scan_result_loop(rx: mpsc::Receiver<ScanResult>, handle: AppHandle) {
        thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                if let Err(e) = handle.emit("scan-result", &result) {
                    eprintln!("向前端推送扫描结果失败: {e}");
                }
            }
        });
    }

    /// 向前端推送当前状态。
    fn emit_status(&self) {
        let status = self.get_status();
        if let Err(e) = self.handle.emit("app-status", &status) {
            error!("向前端推送状态失败: {e}");
        }
    }
}
