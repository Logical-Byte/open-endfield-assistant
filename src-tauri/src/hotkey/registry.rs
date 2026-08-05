//! 热键匹配工具（基础设施，通用库）。
//!
//! 定义热键绑定 [`HotkeyBinding`] 与命中判断 [`binding_matches`]，
//! 只回答"一次按键事件是否命中某个绑定"：
//! - 只响应按下（弹起事件不参与匹配）；
//! - 虚拟键码一致且修饰键组合完全一致才命中（无修饰符的绑定要求
//!   Alt / Ctrl / Shift / Win 均未按下）；
//! - **不做任何放行过滤**：前台窗口判断、重复触发抑制等"该不该响应"的规则
//!   一律留给应用层（见 [`crate::controller`]）处理。

use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

use super::listener::KeyEvent;

/// 单个热键绑定：虚拟键码 + 修饰符（无身份字段，身份由注册表内部管理）。
#[derive(Clone, Copy)]
pub struct HotkeyBinding {
    /// 虚拟键码（如 `VK_OEM_1`）
    pub vk: u32,
    /// 修饰键（如 `MOD_ALT`），无修饰符时传 0
    pub modifiers: u32,
}

/// 修饰键掩码：只比较 Alt / Ctrl / Shift / Win 四位的组合
/// （`MOD_NOREPEAT` 等非修饰标志不参与匹配；按住重复已由第一层过滤）。
const MOD_MASK: u32 = MOD_ALT.0 | MOD_CONTROL.0 | MOD_SHIFT.0 | MOD_WIN.0;

/// 判断一次按键事件是否命中绑定。
///
/// 无修饰符的绑定要求 Alt / Ctrl / Shift / Win 均未按下；
/// 带修饰符的绑定要求对应修饰键按下且其余修饰键未按下。
/// 第一层 [`KeyEvent::mods`] 的位布局与 `MOD_*` 常量一致，可直接比较。
pub fn binding_matches(key_event: &KeyEvent, expected_hotkey_binding: &HotkeyBinding) -> bool {
    // 只响应按下（弹起事件不参与匹配）；虚拟键码一致且修饰键组合完全一致才命中
    key_event.down
        && expected_hotkey_binding.vk == key_event.vk
        && key_event.mods == expected_hotkey_binding.modifiers & MOD_MASK
}
