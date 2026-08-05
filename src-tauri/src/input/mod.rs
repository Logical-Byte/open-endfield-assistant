//! 输入器模块（基础设施，通用库）。
//!
//! 通过 [`InputBase`] trait 抽象输入能力，`Session` 持有 `Box<dyn InputBase>`，
//! 运行时可切换（见 `Session::set_input`）。当前生产使用 [`SeizeInput`]。
//!
//! - [`SeizeInput`]：基于 `SendInput` / `mouse_event` 的物理输入
//!   （鼠标移动 / 点击 / 滚轮 / 键盘），操作前自动确保窗口在前台；
//!   `block_input` 可在操作期间屏蔽真实键盘鼠标；
//! - [`Contact`]：鼠标按键枚举（左 / 右 / 中 / X1 / X2）。

mod base;
mod input_utils;
mod seize;

pub use base::*;
pub use input_utils::*;
pub use seize::*;
