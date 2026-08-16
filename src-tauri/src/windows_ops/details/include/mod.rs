//! Win32 原始宏定义（基础设施）。
//!
//! 提供 `windows` crate 未覆盖的少量 Win32 宏（`MAKELONG!` / `MAKEWPARAM!` / `MAKELPARAM!`），
//! 经 `#[macro_export]` 在 crate 根可见，主要供 `input` 层拼装鼠标消息参数使用。

mod minwindef;
mod winuser;
