//! 第二层：热键注册器（基础设施，通用库）。
//!
//! 基于第一层 [`crate::hotkey::listener`] 的原始键盘事件流，提供"逐个注册热键 →
//! 命中即发事件"的能力：
//! - [`HotkeyRegistry::new`] 启动**一条**共享监听（第一层）+ 一个匹配线程，
//!   所有热键共用，不会重复安装钩子；
//! - [`HotkeyRegistry::register_hotkey`] 一次只注册一个热键，返回**该热键专属的
//!   事件流**。热键身份由注册表内部管理（事件流通道即身份），调用方无需提供
//!   tag，天然不存在"重复 tag"导致的事件混淆；
//! - **不做任何放行过滤**：前台窗口判断、重复触发抑制等"该不该响应"的规则
//!   一律留给第三层（应用层）处理；
//! - 不知道按下后要干什么：应用层把每条事件流绑定到自己的动作即可。
//!
//! 注册表句柄只在注册阶段需要：内部匹配线程持有共享状态的克隆，句柄丢弃后
//! 事件仍持续产生（应用在启动时注册完毕即可安全丢弃句柄）。

use std::sync::{Arc, RwLock, mpsc};
use std::thread;

use anyhow::Result;
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

use super::listener::{KeyEvent, listen};

/// 单个热键绑定：虚拟键码 + 修饰符（无身份字段，身份由注册表内部管理）。
#[derive(Clone, Copy)]
pub struct HotkeyBinding {
    /// 虚拟键码（如 `VK_OEM_1`）
    pub vk: u32,
    /// 修饰键（如 `MOD_ALT`），无修饰符时传 0
    pub modifiers: u32,
}

/// 一次热键触发事件（命中通知）。
///
/// 身份不放在事件里：每条事件流对应**一个**已注册的热键，
/// "命中哪个热键"由所属事件流隐含。
#[derive(Debug, Clone, Copy)]
pub struct HotkeyEvent;

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

/// 注册表内部条目：绑定 + 专属事件发送端（通道即身份）。
struct HotkeyEntry {
    binding: HotkeyBinding,
    tx: mpsc::Sender<HotkeyEvent>,
}

/// 共享热键注册表（第二层）。
///
/// 内部持有一条共享监听（第一层）与一个匹配线程，所有热键共用；
/// 提供逐个注册热键的能力，每个热键返回一条专属事件流。
pub struct HotkeyRegistry {
    /// 绑定表：注册线程写（仅在注册阶段）、匹配线程读（每次按键）。
    /// 写入极少且无竞争，读锁每次按键短暂持有，开销可忽略。
    bindings: Arc<RwLock<Vec<HotkeyEntry>>>,
}

impl HotkeyRegistry {
    /// 启动共享监听与匹配线程。
    pub fn new() -> Result<Self> {
        let key_rx = listen()?;
        let bindings: Arc<RwLock<Vec<HotkeyEntry>>> = Arc::new(RwLock::new(Vec::new()));

        let bindings_thread = Arc::clone(&bindings);
        thread::spawn(move || {
            while let Ok(key) = key_rx.recv() {
                let guard = bindings_thread.read().unwrap();
                let Some(entry) = guard.iter().find(|e| binding_matches(&e.binding, &key)) else {
                    continue;
                };
                let _ = entry.tx.send(HotkeyEvent);
            }
        });

        Ok(Self { bindings })
    }

    /// 注册一个热键（第二层）。
    ///
    /// 一次只注册一个，返回该热键**专属的事件流**；多个热键各自独立，
    /// 互不影响。身份由注册表内部管理（通道即身份），无需调用方提供 tag，
    /// 也不会出现"重复 tag"导致的事件混淆。
    pub fn register_hotkey(&self, binding: HotkeyBinding) -> mpsc::Receiver<HotkeyEvent> {
        let (tx, rx) = mpsc::channel();
        self.bindings
            .write()
            .unwrap()
            .push(HotkeyEntry { binding, tx });
        rx
    }
}
