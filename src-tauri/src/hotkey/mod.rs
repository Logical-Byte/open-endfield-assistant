//! 全局热键模块。
//!
//! 拆分为两层：
//! - **基础设施层** [`registry`]：通用"注册 / 监听 / 过滤"（`WH_KEYBOARD_LL`，只感知不拦截），
//!   不知道按下后要干什么；
//! - **应用层**：键位 → 动作的绑定表与分发逻辑在 [`crate::controller`]。

mod registry;

pub use registry::{HotkeyBinding, HotkeyEvent, HotkeyFilter, HotkeyRegistry};
