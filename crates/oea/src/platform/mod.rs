//! 平台原语的所有权根。
//!
//! 目标窗口能力由独立的 `oea-window` crate 提供。其余公开 topic 模块定义 OEA 使用的接口。
//! Windows API 调用及相关实现集中在私有的
//! `windows` 区域，原生类型不会通过公开接口泄漏给业务代码。

#[cfg(target_os = "windows")]
mod windows;

pub mod admin;
pub mod data_protection;
pub mod dialog;
pub mod file;
pub mod hotkey;
pub mod locale;
pub mod proxy;
pub mod update;
pub mod webview;
pub mod window;

#[cfg(unix)]
fn unsupported(operation: &str) -> anyhow::Error {
    anyhow::anyhow!("{operation} is not supported on {}", std::env::consts::OS)
}
