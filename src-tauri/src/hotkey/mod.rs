//! 全局热键模块。
//!
//! 职责严格分离（过滤逻辑不属于热键本身）：
//! - `listener`：原始键盘监听（`WH_KEYBOARD_LL`，只感知不拦截），过滤按住自动重复
//!   （等价 `MOD_NOREPEAT` 语义），只发"按下 / 弹起"事件，广播到一条事件流；
//! - `registry`：定义 [`HotkeyBinding`] 与命中判断 [`binding_matches`]，纯匹配工具，
//!   不做任何放行过滤；
//! - 应用层（见 [`crate::controller`]）：消费事件流，做前台窗口过滤与动作分发。
//!
//! 监听层在按键按下时立即发事件；"该不该响应"由应用层决定。

mod listener;
mod registry;

pub use listener::{KeyEvent, listen};
pub use registry::{HotkeyBinding, binding_matches};
