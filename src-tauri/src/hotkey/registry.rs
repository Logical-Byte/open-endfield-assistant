//! 第二层：热键注册器（基础设施，通用库）。
//!
//! 基于第一层 [`crate::hotkey::listener`] 的原始键盘事件流，提供"注册键位 →
//! 命中即发事件"的能力：
//! - 通过 [`HotkeyBinding`]（虚拟键码 + 修饰符）注册，命中时发出 [`HotkeyEvent`]；
//! - **不做任何放行过滤**：前台窗口判断、重复触发抑制等"该不该响应"的规则
//!   一律留给第三层（应用层）处理；
//! - 不知道按下后要干什么：应用层通过 `tag` 映射为具体动作。

use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

use super::listener::{KeyEvent, listen};

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

/// 修饰键掩码：只比较 Alt / Ctrl / Shift / Win 四位的组合
/// （`MOD_NOREPEAT` 等非修饰标志不参与匹配；按住重复已由第一层过滤）。
const MOD_MASK: u32 = MOD_ALT.0 | MOD_CONTROL.0 | MOD_SHIFT.0 | MOD_WIN.0;

/// 判断一次按键事件是否命中绑定。
///
/// 无修饰符的绑定要求 Alt / Ctrl / Shift / Win 均未按下；
/// 带修饰符的绑定要求对应修饰键按下且其余修饰键未按下。
/// 第一层 [`KeyEvent::mods`] 的位布局与 `MOD_*` 常量一致，可直接比较。
fn binding_matches(binding: &HotkeyBinding, key: &KeyEvent) -> bool {
    // 只响应按下（弹起事件不参与匹配）；虚拟键码一致且修饰键组合完全一致才命中
    key.down && binding.vk == key.vk && key.mods as u32 == binding.modifiers & MOD_MASK
}

/// 注册热键（第二层）。
///
/// 内部启动第一层键盘监听，把按键流与绑定表匹配，命中即发事件。
/// 不做任何放行过滤；过滤（如前台窗口规则）由第三层（应用层）负责。
///
/// # 返回
/// 事件接收端 `Receiver<HotkeyEvent>`：由调用方 move 进消费线程，用阻塞 `recv`
/// 接收事件（消息驱动，无轮询、无锁）。
pub fn register_hotkey(bindings: &[HotkeyBinding]) -> Result<mpsc::Receiver<HotkeyEvent>> {
    let key_rx = listen()?;
    let bindings = bindings.to_vec();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        while let Ok(key) = key_rx.recv() {
            let Some(binding) = bindings.iter().find(|b| binding_matches(b, &key)) else {
                continue;
            };
            let _ = tx.send(HotkeyEvent { tag: binding.tag });
        }
    });

    Ok(rx)
}
