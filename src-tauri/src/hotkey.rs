//! 全局快捷键监听模块。
//!
//! 在独立线程中通过低级键盘钩子（`WH_KEYBOARD_LL`）统一感知所有热键，将热键
//! 事件通过 [`std::sync::mpsc`] 通道发送给主线程。空闲时主线程通过
//! [`HotkeyListener::wait_event`] 阻塞等待事件；当主任务正在运行时，主线程忙于
//! 执行任务而无法读取通道，此时再按"切换主任务"或"退出程序"热键会直接设置
//! 停止标志，由任务内部轮询该标志实现优雅停止。
//!
//! 钩子只感知按键、从不拦截（一律透传给前台程序，不影响其它程序打字）：
//! - 分号 / 引号仅在 OEA 窗口或终末地游戏窗口位于前台时响应；
//! - `Alt+Delete` 退出热键全局生效，不受前台窗口限制。

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use anyhow::{Result, anyhow};
use scopeguard::defer;
use tracing::{error, info};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_CONTROL, VK_LWIN, VK_MENU,
    VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

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

/// 低级键盘钩子（`WH_KEYBOARD_LL`）所需的线程本地状态。
///
/// 钩子回调由系统在安装钩子的线程（即热键监听线程）的消息泵中调用，
/// 因此所有状态只会在该线程上被访问，使用线程本地存储即可，无需加锁。
struct HookState {
    /// 事件发送端（热键线程 → 主线程）
    tx: mpsc::Sender<HotkeyEvent>,
    /// 停止标志：主任务运行时，热键请求停止
    stop_flag: Arc<AtomicBool>,
    /// 主任务是否正在运行
    running: Arc<AtomicBool>,
    /// 前台窗口守卫：分号/引号仅在 OEA 或终末地窗口在前台时响应
    foreground: ForegroundGuard,
    /// 热键绑定列表
    bindings: Vec<HotkeyBinding>,
    /// 当前按住未弹起的虚拟键码，用于抑制按住时的自动重复触发
    pressed: Vec<u32>,
    /// 修饰键按下状态（基于钩子事件流自跟踪）：位0=Alt，位1=Ctrl，位2=Shift，位3=Win
    mods: u8,
}

impl HookState {
    /// 判断按键是否与绑定匹配（虚拟键码 + 修饰键状态）。
    ///
    /// 无修饰符的绑定（分号/引号）要求 Alt/Ctrl/Shift/Win 均未按下；
    /// 带修饰符的绑定（如 `Alt+Delete`）要求对应修饰键按下且其余修饰键未按下。
    fn binding_matches(&self, binding: &HotkeyBinding, kb: &KBDLLHOOKSTRUCT) -> bool {
        if binding.vk != kb.vkCode {
            return false;
        }

        // Alt 用钩子自带的 LLKHF_ALTDOWN 标志（系统直接给出，最可靠），并辅以自跟踪状态
        // 兜底；Ctrl/Shift/Win 用钩子事件流自跟踪的状态，避免在钩子回调中依赖
        // GetAsyncKeyState 的异步状态（该 API 在钩子回调中不可靠）。
        let alt_down = kb.flags.0 & LLKHF_ALTDOWN.0 != 0 || self.mods & (1 << 0) != 0;
        let ctrl_down = self.mods & (1 << 1) != 0;
        let shift_down = self.mods & (1 << 2) != 0;
        let win_down = self.mods & (1 << 3) != 0;

        let want_alt = binding.modifiers.0 & MOD_ALT.0 != 0;
        let want_ctrl = binding.modifiers.0 & MOD_CONTROL.0 != 0;
        let want_shift = binding.modifiers.0 & MOD_SHIFT.0 != 0;
        let want_win = binding.modifiers.0 & MOD_WIN.0 != 0;

        alt_down == want_alt
            && ctrl_down == want_ctrl
            && shift_down == want_shift
            && win_down == want_win
    }

    /// 根据 keydown / keyup 更新修饰键跟踪状态。
    fn track_modifier(&mut self, vk: u32, down: bool) {
        let bit: u8 = match vk {
            v if v == VK_MENU.0 as u32 => 1 << 0,    // Alt
            v if v == VK_CONTROL.0 as u32 => 1 << 1, // Ctrl
            v if v == VK_SHIFT.0 as u32 => 1 << 2,   // Shift
            v if v == VK_LWIN.0 as u32 || v == VK_RWIN.0 as u32 => 1 << 3, // Win
            _ => return,
        };
        if down {
            self.mods |= bit;
        } else {
            self.mods &= !bit;
        }
    }

    /// 分发一次按键按下：匹配绑定 → 按前台规则判断 → 触发对应事件。
    fn on_key_down(&mut self, kb: &KBDLLHOOKSTRUCT) {
        let Some(binding) = self.bindings.iter().find(|b| self.binding_matches(b, kb)) else {
            return;
        };

        // 前台规则：分号/引号仅在 OEA 或终末地窗口位于前台时响应；退出热键全局生效
        let needs_foreground = !matches!(binding.event, HotkeyEvent::ExitProgram);
        if needs_foreground && !self.foreground.is_foreground_eligible() {
            info!(
                "忽略热键 {:?}：前台窗口不是 OEA 或终末地窗口",
                binding.event
            );
            return;
        }

        match binding.event {
            HotkeyEvent::ToggleMainTask => {
                // 主任务运行中 → 设置停止标志，由任务内部轮询停止
                if self.running.load(Ordering::Relaxed) {
                    self.stop_flag.store(true, Ordering::Relaxed);
                    info!("收到停止请求（引号键），正在停止主任务...");
                } else {
                    // 空闲 → 通知主线程启动主任务
                    let _ = self.tx.send(HotkeyEvent::ToggleMainTask);
                }
            }
            HotkeyEvent::ScanSingleArchive => {
                // 主任务运行中忽略单次扫描，避免命令积压到任务结束后才执行
                if !self.running.load(Ordering::Relaxed) {
                    let _ = self.tx.send(HotkeyEvent::ScanSingleArchive);
                }
            }
            HotkeyEvent::ExitProgram => {
                // 主任务运行中先请求停止，任务结束后主线程再读取退出事件
                if self.running.load(Ordering::Relaxed) {
                    self.stop_flag.store(true, Ordering::Relaxed);
                    info!("收到退出请求（Alt+Delete），正在停止主任务...");
                }
                let _ = self.tx.send(HotkeyEvent::ExitProgram);
            }
        }
    }
}

thread_local! {
    static HOOK_STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

/// `WH_KEYBOARD_LL` 低级键盘钩子回调。
///
/// 钩子只感知按键、从不拦截：匹配到绑定并符合前台规则时分发事件，
/// 但一律调用 `CallNextHookEx` 放行按键，保证在其它程序里正常输入。
unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    // 无论是否处理，按键都透传给前台程序（不拦截）
    let next = unsafe { CallNextHookEx(None, n_code, w_param, l_param) };

    if n_code != HC_ACTION as i32 {
        return next;
    }

    let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
    let vk = kb.vkCode;

    // 回调运行在安装钩子的线程（消息泵）中，线程本地状态无并发访问
    HOOK_STATE.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        let Some(state) = borrowed.as_mut() else {
            return;
        };

        match w_param.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // 按住自动重复：同一 vk 未弹起前不再触发
                if state.pressed.contains(&vk) {
                    return;
                }
                state.pressed.push(vk);
                state.track_modifier(vk, true);
                state.on_key_down(kb);
            }
            WM_KEYUP | WM_SYSKEYUP => {
                state.pressed.retain(|&v| v != vk);
                state.track_modifier(vk, false);
            }
            _ => {}
        }
    });

    next
}

/// 全局快捷键监听器。
///
/// 通过低级键盘钩子统一感知热键，后台线程持续泵消息：
/// - 主任务空闲时，热键事件通过通道发送给主线程；
/// - 主任务运行时，按"切换主任务"热键直接设置停止标志（主线程无法读取通道），
///   由任务内部轮询停止标志实现优雅停止；
/// - 主任务运行时忽略"单次扫描"热键，避免命令积压到任务结束后才被执行。
/// - 钩子不拦截任何按键，一律透传给前台程序。
pub struct HotkeyListener {
    /// 事件接收端（主线程等待热键事件）
    rx: mpsc::Receiver<HotkeyEvent>,
    /// 停止标志：主任务运行时，热键线程设置此标志请求停止
    stop_flag: Arc<AtomicBool>,
    /// 主任务是否正在运行（热键线程据此决定"切换"热键的行为）
    main_running: Arc<AtomicBool>,
}

impl HotkeyListener {
    /// 安装低级键盘钩子并启动后台监听线程。
    ///
    /// # 参数
    /// - `bindings`: 热键绑定列表。
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
            // 安装低级键盘钩子（统一感知所有热键，不拦截任何按键）。
            // 钩子回调运行在安装线程的消息泵中，故用线程本地存储传递状态。
            let binding_count = bindings.len();
            let hook_state = HookState {
                tx,
                stop_flag: flag,
                running,
                foreground,
                bindings,
                pressed: Vec::new(),
                mods: 0,
            };
            HOOK_STATE.with(|cell| *cell.borrow_mut() = Some(hook_state));
            let hook = match unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            } {
                Ok(hook) => {
                    info!("低级键盘钩子安装成功（{binding_count} 个热键）");
                    Some(hook)
                }
                Err(e) => {
                    error!("低级键盘钩子安装失败: {e}");
                    None
                }
            };
            let Some(hook) = hook else {
                HOOK_STATE.with(|cell| *cell.borrow_mut() = None);
                return;
            };

            // 线程退出时自动清理：卸载钩子、清空线程本地状态
            defer! {
                let _ = unsafe { UnhookWindowsHookEx(hook) };
                HOOK_STATE.with(|cell| *cell.borrow_mut() = None);
            }

            // 消息循环：泵出低级键盘钩子消息（钩子回调由系统在此线程的消息泵中调用）。
            // 其它消息无需处理，仅保持消息泵持续运行。
            let mut msg = MSG::default();
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                // GetMessageW 返回 0（收到 WM_QUIT）或 -1（出错）时退出
                if ret.0 == 0 || ret.0 == -1 {
                    break;
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
