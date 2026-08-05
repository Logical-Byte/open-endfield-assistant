//! 全局热键注册与监听（基础设施，通用库）。
//!
//! 只提供"注册键位 + 监听事件"的通用能力，**不知道**按下后要干什么：
//! - 应用层通过 [`HotkeyBinding`] 的 `tag` 区分键位，自行映射为动作；
//! - 是否放行由应用层提供过滤回调（如"仅 OEA / 游戏窗口在前台时响应"）。
//!
//! 钩子只感知按键、从不拦截（一律透传，不影响其它程序打字）。

use std::cell::RefCell;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use anyhow::{Result, anyhow};
use scopeguard::defer;
use tracing::{error, info};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// 单个热键绑定：虚拟键码 + 修饰符 → 应用自定义标签。
#[derive(Clone, Copy)]
pub struct HotkeyBinding {
    /// 虚拟键码（如 `VK_OEM_1`）
    pub vk: u32,
    /// 修饰键（如 `MOD_ALT`），无修饰符时传 0
    pub modifiers: u32,
    /// 应用自定义标签（应用层映射为具体动作）
    pub tag: u32,
}

/// 一次热键触发事件：只报"哪个键被按"（tag），不报"该干嘛"。
#[derive(Debug, Clone, Copy)]
pub struct HotkeyEvent {
    /// 命中的绑定标签
    pub tag: u32,
}

/// 按键放行过滤回调：返回 `true` 才分发事件（应用层实现具体规则）。
///
/// 用 `Box` 而非 `Arc`：回调只存活在钩子线程内（单所有者），且
/// `Box<dyn Fn + Send>` 满足 `Send`（`Arc<dyn Fn + Send>` 因 `Arc` 要求 `T: Sync` 而不满足）。
pub type HotkeyFilter = Box<dyn Fn(u32) -> bool + Send>;

/// 低级键盘钩子所需的线程本地状态。
///
/// 回调由系统在安装钩子的线程（消息泵）中调用，因此所有状态只会在该线程
/// 上被访问，使用线程本地存储即可，无需加锁。
struct HookState {
    /// 事件发送端（钩子线程 → 轮询线程）
    tx: mpsc::Sender<HotkeyEvent>,
    /// 热键绑定列表
    bindings: Vec<HotkeyBinding>,
    /// 可选放行过滤回调
    filter: Option<HotkeyFilter>,
    /// 当前按住未弹起的虚拟键码（抑制按住时的自动重复）
    pressed: Vec<u32>,
    /// 修饰键按下状态（基于钩子事件流自跟踪）：位0=Alt，位1=Ctrl，位2=Shift，位3=Win
    mods: u8,
}

impl HookState {
    /// 判断按键是否与绑定匹配（虚拟键码 + 修饰键状态）。
    ///
    /// 无修饰符的绑定要求 Alt/Ctrl/Shift/Win 均未按下；
    /// 带修饰符的绑定要求对应修饰键按下且其余修饰键未按下。
    fn binding_matches(&self, binding: &HotkeyBinding, kb: &KBDLLHOOKSTRUCT) -> bool {
        if binding.vk != kb.vkCode {
            return false;
        }

        // Alt 用钩子自带的 LLKHF_ALTDOWN 标志（系统直接给出，最可靠），并辅以自跟踪状态
        // 兜底；Ctrl/Shift/Win 用钩子事件流自跟踪的状态（GetAsyncKeyState 在钩子回调中不可靠）。
        let alt_down = kb.flags.0 & LLKHF_ALTDOWN.0 != 0 || self.mods & (1 << 0) != 0;
        let ctrl_down = self.mods & (1 << 1) != 0;
        let shift_down = self.mods & (1 << 2) != 0;
        let win_down = self.mods & (1 << 3) != 0;

        let want_alt = binding.modifiers & MOD_ALT.0 != 0;
        let want_ctrl = binding.modifiers & MOD_CONTROL.0 != 0;
        let want_shift = binding.modifiers & MOD_SHIFT.0 != 0;
        let want_win = binding.modifiers & MOD_WIN.0 != 0;

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

    /// 分发一次按键按下：匹配绑定 → 过滤回调 → 发事件。
    fn on_key_down(&mut self, kb: &KBDLLHOOKSTRUCT) {
        let Some(binding) = self.bindings.iter().find(|b| self.binding_matches(b, kb)) else {
            return;
        };

        if let Some(filter) = &self.filter {
            if !filter(binding.tag) {
                info!("忽略热键 tag={}：不满足放行条件", binding.tag);
                return;
            }
        }

        let _ = self.tx.send(HotkeyEvent { tag: binding.tag });
    }
}

thread_local! {
    static HOOK_STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

/// `WH_KEYBOARD_LL` 低级键盘钩子回调：只感知不拦截。
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

/// 全局热键注册与监听器。
pub struct HotkeyRegistry {
    /// 事件接收端（轮询线程非阻塞读取）
    rx: mpsc::Receiver<HotkeyEvent>,
}

impl HotkeyRegistry {
    /// 安装低级键盘钩子并启动监听线程。
    ///
    /// # 参数
    /// - `bindings`: 热键绑定列表
    /// - `filter`: 可选的放行回调（如前台窗口过滤）
    pub fn new(bindings: &[HotkeyBinding], filter: Option<HotkeyFilter>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let bindings = bindings.to_vec();

        thread::spawn(move || {
            // 安装低级键盘钩子（统一感知所有热键，不拦截任何按键）。
            // 钩子回调运行在安装线程的消息泵中，故用线程本地存储传递状态。
            let hook_state = HookState {
                tx,
                bindings,
                filter,
                pressed: Vec::new(),
                mods: 0,
            };
            HOOK_STATE.with(|cell| *cell.borrow_mut() = Some(hook_state));

            let hook =
                match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) }
                {
                    Ok(hook) => {
                        info!("低级键盘钩子安装成功");
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

            // 消息循环：泵出低级键盘钩子消息（回调由系统在此线程的消息泵中调用）
            let mut msg = MSG::default();
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 == 0 || ret.0 == -1 {
                    break;
                }
            }
        });

        Ok(Self { rx })
    }

    /// 非阻塞地取下一个热键事件。
    pub fn try_next(&self) -> Result<Option<HotkeyEvent>> {
        match self.rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(_) => Err(anyhow!("热键监听线程已退出")),
        }
    }
}
