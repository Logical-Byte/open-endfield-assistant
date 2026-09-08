//! 应用 WebView 的平台接口。

use std::path::Path;

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::windows;

/// 确保应用所需的 WebView Runtime 已安装。
///
/// 已安装或安装成功时返回 `true`；用户拒绝或安装失败时返回 `false`。
/// macOS 开发外壳直接返回 `true`。
pub fn ensure_installed(cache_dir: &Path) -> Result<bool> {
    #[cfg(target_os = "windows")]
    {
        windows::webview2::ensure_installed(cache_dir)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = cache_dir;
        Ok(true)
    }
}

/// 读取 WebView 当前缩放因子；macOS 开发外壳返回 `1.0`。
pub fn get_zoom(window: tauri::WebviewWindow) -> Result<f64> {
    #[cfg(target_os = "windows")]
    {
        windows::webview2::get_zoom(window)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window;
        Ok(1.0)
    }
}

/// 注册 WebView 原生缩放变化监听；macOS 开发外壳不执行任何操作。
pub fn register_zoom_changed_listener(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        windows::webview2::register_zoom_changed_listener(window);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = window;
    }
}
