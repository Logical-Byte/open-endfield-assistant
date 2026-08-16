//! 第一层：原始键盘监听器（基础设施，通用库）。
//!
//! 类似 `rdev::listen`：监听**所有**键盘消息（按下 / 弹起），只感知不拦截，
//! 一律透传给其它程序（不影响打字）。按住按键时的自动重复已在监听层过滤，
//! 等价于 `RegisterHotKey` 的 `MOD_NOREPEAT` 语义，下游只会收到"真正按下"与
//! "弹起"两类事件。
//!
//! 本层不知道热键、不知道动作，只负责把原始键盘事件广播到通道。

use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use scopeguard::defer;
use tracing::{error, info};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_DELETE, VK_LWIN, VK_MENU, VK_OEM_7, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// 修饰键状态位（私有；`KeyEvent::mods` 的位布局与 `RegisterHotKey` 的
/// `MOD_*` 常量一致：Alt=1、Ctrl=2、Shift=4、Win=8，便于上层直接比较）。
const MOD_ALT: u32 = 1 << 0;
const MOD_CTRL: u32 = 1 << 1;
const MOD_SHIFT: u32 = 1 << 2;
const MOD_WIN: u32 = 1 << 3;

/// Delete 键的 OEA 键码。
pub const DELETE_KEY: u32 = VK_DELETE.0 as u32;
/// `'` 键的 OEA 键码。
pub const OEM_7_KEY: u32 = VK_OEM_7.0 as u32;
/// Alt 修饰键的 OEA 状态位。
pub const ALT_MODIFIER: u32 = MOD_ALT;

/// 一次原始键盘事件。
///
/// - `down = true`：按键被按下（首次按下，自动重复已过滤）；
/// - `down = false`：按键弹起。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEvent {
    /// 虚拟键码（如 `VK_OEM_1`）
    pub vk: u32,
    /// 按下（true）或弹起（false）
    pub down: bool,
    /// 修饰键状态快照：位 0=Alt、位 1=Ctrl、位 2=Shift、位 3=Win
    /// （位布局与 `MOD_*` 常量一致，可直接比较）
    pub modifiers: u32,
}

/// 钩子线程本地状态（回调由系统在安装钩子的线程消息泵中调用，无并发访问）。
struct ListenerState {
    /// 事件发送端（钩子线程 → 上层消费）
    tx: mpsc::Sender<KeyEvent>,
    /// 当前按住未弹起的虚拟键码（抑制按住时的自动重复）
    pressed: Vec<u32>,
    /// 修饰键按下状态（基于钩子事件流自跟踪）
    mods: u32,
}

thread_local! {
    static LISTENER_STATE: RefCell<Option<ListenerState>> = const { RefCell::new(None) };
}

/// 计算当前事件对应的修饰键状态快照。
///
/// Alt 用钩子自带的 `LLKHF_ALTDOWN` 标志（系统直接给出，最可靠），并辅以自跟踪
/// 状态兜底；Ctrl/Shift/Win 用钩子事件流自跟踪的状态（`GetAsyncKeyState` 在钩子
/// 回调中不可靠）。
fn mods_snapshot(kb: &KBDLLHOOKSTRUCT, tracked: u32) -> u32 {
    let alt = kb.flags.0 & LLKHF_ALTDOWN.0 != 0 || tracked & MOD_ALT != 0;
    (if alt { MOD_ALT } else { 0 }) | (tracked & (MOD_CTRL | MOD_SHIFT | MOD_WIN))
}

/// 根据 keydown / keyup 更新修饰键跟踪状态。
fn track_modifier(mods: &mut u32, vk: u32, down: bool) {
    let bit: u32 = match vk {
        v if v == VK_MENU.0 as u32 => MOD_ALT,     // Alt
        v if v == VK_CONTROL.0 as u32 => MOD_CTRL, // Ctrl
        v if v == VK_SHIFT.0 as u32 => MOD_SHIFT,  // Shift
        v if v == VK_LWIN.0 as u32 || v == VK_RWIN.0 as u32 => MOD_WIN, // Win
        _ => return,
    };
    if down {
        *mods |= bit;
    } else {
        *mods &= !bit;
    }
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
    LISTENER_STATE.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        let Some(state) = borrowed.as_mut() else {
            return;
        };

        match w_param.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // 按住自动重复（等价 MOD_NOREPEAT）：同一 vk 未弹起前不再发事件
                if state.pressed.contains(&vk) {
                    return;
                }
                state.pressed.push(vk);
                track_modifier(&mut state.mods, vk, true);
                let _ = state.tx.send(KeyEvent {
                    vk,
                    down: true,
                    modifiers: mods_snapshot(kb, state.mods),
                });
            }
            WM_KEYUP | WM_SYSKEYUP => {
                state.pressed.retain(|&v| v != vk);
                track_modifier(&mut state.mods, vk, false);
                let _ = state.tx.send(KeyEvent {
                    vk,
                    down: false,
                    modifiers: mods_snapshot(kb, state.mods),
                });
            }
            _ => {}
        }
    });

    next
}

/// 启动原始键盘监听（第一层）。
///
/// 安装 `WH_KEYBOARD_LL` 低级键盘钩子，把键盘消息（按下 / 弹起，自动重复已过滤）
/// 广播到返回的接收端。钩子只感知、不拦截任何按键。
pub fn listen() -> Result<mpsc::Receiver<KeyEvent>> {
    let (tx, rx) = mpsc::channel();

    thread::Builder::new()
        .name("oea-keyboard".to_string())
        .spawn(move || {
            // 安装低级键盘钩子（统一感知所有按键，不拦截任何按键）。
            // 钩子回调运行在安装线程的消息泵中，故用线程本地存储传递状态。
            let listener_state = ListenerState {
                tx,
                pressed: Vec::new(),
                mods: 0,
            };
            LISTENER_STATE.with(|cell| *cell.borrow_mut() = Some(listener_state));

            let hook = match unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            } {
                Ok(hook) => {
                    info!("键盘监听钩子安装成功");
                    Some(hook)
                }
                Err(e) => {
                    error!("键盘监听钩子安装失败: {e}");
                    None
                }
            };
            let Some(hook) = hook else {
                LISTENER_STATE.with(|cell| *cell.borrow_mut() = None);
                return;
            };

            // 线程退出时自动清理：卸载钩子、清空线程本地状态
            defer! {
                let _ = unsafe { UnhookWindowsHookEx(hook) };
                LISTENER_STATE.with(|cell| *cell.borrow_mut() = None);
            }

            // 消息循环：泵出低级键盘钩子消息（回调由系统在此线程的消息泵中调用）
            let mut msg = MSG::default();
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 == 0 || ret.0 == -1 {
                    break;
                }
            }
        })
        .expect("启动键盘监听线程失败");

    Ok(rx)
}
