//! 全局快捷键监听模块。
//!
//! 在独立线程中注册并持续监听多个热键，将热键事件通过 [`std::sync::mpsc`] 通道
//! 发送给主线程。空闲时主线程通过 [`HotkeyListener::wait_event`] 阻塞等待事件；
//! 当主任务正在运行时，主线程忙于执行任务而无法读取通道，此时再按"切换主任务"
//! 或"退出程序"热键会直接设置停止标志，由任务内部轮询该标志实现优雅停止。
//!
//! 前台窗口规则：分号 / 引号热键仅在 OEA 窗口或终末地游戏窗口位于前台时响应，
//! 其它窗口位于前台时仅打印日志而不响应；`Alt+Delete` 退出热键全局生效。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use anyhow::{Result, anyhow};
use scopeguard::defer;
use tracing::{error, info};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

use crate::window::ForegroundGuard;

/// 热键触发的事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// 切换主任务运行状态（引号键 `'`）：
    /// - 空闲时按下 → 启动主任务
    /// - 主任务运行时按下 → 请求停止主任务
    ToggleMainTask,
    /// 单次扫描当前档案详情（分号键 `;`）：仅截屏识别，不做任何输入操作。
    ScanSingleArchive,
    /// 退出程序（Alt+Delete 键）：优雅停止主任务后退出脚本。
    ExitProgram,
}

/// 单个热键绑定：虚拟键码 + 修饰符 → 触发的事件。
#[derive(Clone, Copy)]
pub struct HotkeyBinding {
    /// 虚拟键码（如 `VK_OEM_1`、`VK_OEM_7`）
    pub vk: u32,
    /// 修饰键（如 `MOD_ALT`），无修饰符时传 `HOT_KEY_MODIFIERS(0)`
    pub modifiers: HOT_KEY_MODIFIERS,
    /// 触发的事件
    pub event: HotkeyEvent,
}

/// 全局快捷键监听器。
///
/// 注册热键后，后台线程持续监听消息队列：
/// - 主任务空闲时，热键事件通过通道发送给主线程；
/// - 主任务运行时，按"切换主任务"热键直接设置停止标志（主线程无法读取通道），
///   由任务内部轮询停止标志实现优雅停止；
/// - 主任务运行时忽略"单次扫描"热键，避免命令积压到任务结束后才被执行。
pub struct HotkeyListener {
    /// 事件接收端（主线程等待热键事件）
    rx: mpsc::Receiver<HotkeyEvent>,
    /// 停止标志：主任务运行时，热键线程设置此标志请求停止
    stop_flag: Arc<AtomicBool>,
    /// 主任务是否正在运行（热键线程据此决定"切换"热键的行为）
    main_running: Arc<AtomicBool>,
}

impl HotkeyListener {
    /// 注册一组热键并启动后台监听线程。
    ///
    /// # 参数
    /// - `bindings`: 热键绑定列表，每个热键在注册时分配唯一的内部 ID。
    /// - `foreground`: 前台窗口守卫，分号 / 引号热键仅在 OEA 或终末地窗口位于前台时响应。
    pub fn new(bindings: &[HotkeyBinding], foreground: ForegroundGuard) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let main_running = Arc::new(AtomicBool::new(false));
        let flag = stop_flag.clone();
        let running = main_running.clone();
        // 转为拥有所有权的数据，以便安全移入监听线程
        let bindings = bindings.to_vec();
        // 前台窗口守卫移入监听线程（热键线程独占访问）
        let foreground = foreground;

        thread::spawn(move || {
            // 依次注册所有热键，记录成功注册的 (id, event)
            let mut registered: Vec<(i32, HotkeyEvent)> = Vec::new();
            for (idx, binding) in bindings.iter().enumerate() {
                // 热键 ID 从 1 开始，WM_HOTKEY 消息的 wParam 携带此 ID
                let id = (idx + 1) as i32;
                match unsafe { RegisterHotKey(None, id, binding.modifiers, binding.vk) } {
                    Ok(()) => {
                        info!("热键注册成功: {:?} (VK=0x{:X})", binding.event, binding.vk);
                        registered.push((id, binding.event));
                    }
                    Err(e) => {
                        error!(
                            "热键注册失败: {:?} (VK=0x{:X}): {e}",
                            binding.event, binding.vk
                        );
                    }
                }
            }

            if registered.is_empty() {
                error!("所有热键均注册失败，监听线程退出");
                return;
            }

            // 线程退出时自动注销所有已注册的热键
            defer! {
                for &(id, _) in &registered {
                    let _ = unsafe { UnregisterHotKey(None, id) };
                }
            }

            // 消息循环：持续监听热键消息
            let mut msg = MSG::default();
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                // GetMessageW 返回 0（收到 WM_QUIT）或 -1（出错）时退出
                if ret.0 == 0 || ret.0 == -1 {
                    break;
                }
                if msg.message != WM_HOTKEY {
                    continue;
                }

                // 通过 wParam 中的热键 ID 找到对应事件
                let id = msg.wParam.0 as i32;
                let Some(&(_, event)) = registered.iter().find(|(rid, _)| *rid == id) else {
                    continue;
                };

                match event {
                    HotkeyEvent::ToggleMainTask => {
                        // 仅当 OEA 或终末地窗口位于前台时才响应，否则只打印日志
                        if !foreground.is_foreground_eligible() {
                            info!("忽略热键 {:?}：前台窗口不是 OEA 或终末地窗口", event);
                        } else if running.load(Ordering::Relaxed) {
                            // 主任务运行中 → 设置停止标志，由任务内部轮询停止
                            flag.store(true, Ordering::Relaxed);
                            info!("收到停止请求（引号键），正在停止主任务...");
                        } else {
                            // 空闲 → 通知主线程启动主任务
                            if tx.send(HotkeyEvent::ToggleMainTask).is_err() {
                                break; // 主线程已退出
                            }
                        }
                    }
                    HotkeyEvent::ScanSingleArchive => {
                        // 仅当 OEA 或终末地窗口位于前台时才响应，否则只打印日志
                        if !foreground.is_foreground_eligible() {
                            info!("忽略热键 {:?}：前台窗口不是 OEA 或终末地窗口", event);
                        } else if !running.load(Ordering::Relaxed) {
                            // 主任务运行中忽略单次扫描，避免命令积压到任务结束后才执行
                            if tx.send(HotkeyEvent::ScanSingleArchive).is_err() {
                                break; // 主线程已退出
                            }
                        }
                    }
                    HotkeyEvent::ExitProgram => {
                        // 退出热键全局生效，不受前台窗口限制
                        // 主任务运行中先请求停止，任务结束后主线程再读取退出事件
                        if running.load(Ordering::Relaxed) {
                            flag.store(true, Ordering::Relaxed);
                            info!("收到退出请求（Alt+Delete），正在停止主任务...");
                        }
                        if tx.send(HotkeyEvent::ExitProgram).is_err() {
                            break; // 主线程已退出
                        }
                    }
                }
            }
        });

        Ok(Self {
            rx,
            stop_flag,
            main_running,
        })
    }

    /// 阻塞等待下一个热键事件。
    ///
    /// 仅当主任务空闲时调用；主任务运行时不要调用此方法
    /// （停止请求通过 [`stop_flag`](Self::stop_flag) 传递）。
    pub fn wait_event(&self) -> Result<HotkeyEvent> {
        self.rx.recv().map_err(|_| anyhow!("热键监听线程已退出"))
    }

    /// 非阻塞地检查是否有待处理的热键事件。
    ///
    /// 返回 `Ok(Some(event))` 表示有事件待处理，`Ok(None)` 表示当前无事件。
    /// 供 Tauri 后台轮询线程使用（避免阻塞持有锁，与扫描线程死锁）。
    pub fn try_wait_event(&self) -> Result<Option<HotkeyEvent>> {
        match self.rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(_) => Err(anyhow!("热键监听线程已退出")),
        }
    }

    /// 获取停止标志的 Arc 克隆，可传递给 [`crate::session::Session`] 用于操作中轮询。
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    /// 重置停止标志（启动新任务或单次扫描前调用，清除上一次的停止信号）。
    pub fn reset_stop(&self) {
        self.stop_flag.store(false, Ordering::Relaxed);
    }

    /// 设置主任务运行状态。
    pub fn set_main_running(&self, running: bool) {
        self.main_running.store(running, Ordering::Relaxed);
    }

    /// 获取主任务运行标志的 Arc 克隆（与外部共享，供 AppController 读写）。
    pub fn main_running_flag(&self) -> Arc<AtomicBool> {
        self.main_running.clone()
    }

    /// 查询主任务是否正在运行。
    pub fn is_main_running(&self) -> bool {
        self.main_running.load(Ordering::Relaxed)
    }
}
