//! 全局热键模块。
//!
//! 拆为三层，职责严格分离（过滤逻辑不属于热键本身）：
//! - **第一层** [`listener`]：原始键盘监听（`WH_KEYBOARD_LL`，只感知不拦截），
//!   过滤按住自动重复（等价 `MOD_NOREPEAT` 语义），只发"按下 / 弹起"事件；
//! - **第二层** [`registry`]：共享热键注册表（[`HotkeyRegistry`]），一条共享监听 +
//!   逐个注册热键，每个热键一条专属事件流，命中即发事件，不做任何放行过滤；
//! - **第三层**（应用层，见 [`crate::controller`]）：做前台窗口过滤与动作分发。
//!
//! 前两层在按键按下时立即做事（监听 / 匹配发事件）；"该不该响应"由应用层决定。

mod listener;
mod registry;

pub use listener::{KeyEvent, listen};
pub use registry::{HotkeyBinding, HotkeyEvent, HotkeyRegistry};
