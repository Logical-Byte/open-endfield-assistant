//! WebView2 Runtime 检测与安装接口。

use std::path::Path;

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::details;

/// 确保 WebView2 Runtime 已安装。
///
/// 已安装或安装成功时返回 `true`；用户拒绝或安装失败时返回 `false`。
#[cfg(target_os = "windows")]
pub fn ensure_installed(cache_dir: &Path) -> Result<bool> {
    details::webview2::ensure_installed(cache_dir)
}

/// WKWebView 不需要 WebView2 Runtime。
#[cfg(target_os = "macos")]
pub fn ensure_installed(_cache_dir: &Path) -> Result<bool> {
    Ok(true)
}
