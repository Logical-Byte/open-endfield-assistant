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

/// 读取 WebView2 当前缩放因子。
#[cfg(target_os = "windows")]
pub fn get_zoom(window: tauri::WebviewWindow) -> Result<f64> {
    details::webview2::get_zoom(window)
}

/// macOS 开发外壳不跟踪 WebView2 缩放，使用默认缩放因子。
#[cfg(target_os = "macos")]
pub fn get_zoom(_window: tauri::WebviewWindow) -> Result<f64> {
    Ok(1.0)
}

/// 注册 WebView2 原生缩放变化监听。
#[cfg(target_os = "windows")]
pub fn register_zoom_changed_listener(window: &tauri::WebviewWindow) {
    details::webview2::register_zoom_changed_listener(window);
}

/// WKWebView 不提供 WebView2 原生缩放事件。
#[cfg(target_os = "macos")]
pub fn register_zoom_changed_listener(_window: &tauri::WebviewWindow) {}
