//! WebView2 Runtime 检测与安装接口。

use std::path::Path;

use anyhow::Result;

use super::details;

/// 确保 WebView2 Runtime 已安装。
///
/// 已安装或安装成功时返回 `true`；用户拒绝或安装失败时返回 `false`。
pub fn ensure_installed(cache_dir: &Path) -> Result<bool> {
    details::webview2::ensure_installed(cache_dir)
}
