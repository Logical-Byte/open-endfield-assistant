//! WebView2 Runtime 检测与自动安装（仅 Windows）。
//!
//! 绿色便携版没有安装程序，Tauri 安装器里"自动下载并安装 WebView2 引导程序"的逻辑
//! 不会执行。本模块在 `main` 入口、Tauri 启动前检测系统是否已安装 WebView2 Runtime，
//! 缺失时用 `reqwest` 从微软官方链接下载 Evergreen Bootstrapper 并安装。
//!
//! 参考：
//! - Tauri Windows Installer 文档（`downloadBootstrapper` 模式）
//! - 微软《Distribute your app and the WebView2 Runtime》

mod detection;
mod install;

pub(in crate::windows_ops) use install::ensure_installed;
