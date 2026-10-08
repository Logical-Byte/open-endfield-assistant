//! 桌面主窗口的运行环境、便携数据目录与原生配置。

use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};
use tracing::warn;

use crate::{app_paths::AppPaths, platform};

/// 确保 WebView 运行环境可用。安装被拒绝或失败时，禁止继续创建窗口。
fn ensure_runtime(cache_dir: &Path) -> Result<()> {
    let installed =
        platform::webview::ensure_installed(cache_dir).inspect_err(|e| warn!("{e:#}"))?;
    ensure!(installed, "WebView2 不可用，安装被取消或失败，无法启动 OEA");
    Ok(())
}

/// 确认 WebView 可用后创建主窗口并注册缩放监听。调用方须先完成更新事务。
pub(super) fn create(app: &tauri::App, app_paths: &AppPaths) -> Result<tauri::WebviewWindow> {
    ensure_runtime(&app_paths.cache_dir())?;

    // 绿色便携：将 WebView2 用户数据保存在应用目录，避免默认写入 `%LOCALAPPDATA%`。
    fs::create_dir_all(app_paths.webview_data_dir()).with_context(|| {
        format!(
            "创建 WebView2 数据目录 {} 失败",
            app_paths.webview_data_dir().display()
        )
    })?;
    // 动态建窗口才能指定便携数据目录，因此不在 `tauri.conf.json` 中声明窗口。
    let builder = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
        .title("OEA")
        .inner_size(1024.0, 640.0)
        .min_inner_size(864.0, 540.0)
        .resizable(true)
        .decorations(false) // 移除系统标题栏
        .shadow(true)
        .data_directory(app_paths.webview_data_dir())
        .zoom_hotkeys_enabled(true); // 允许 Ctrl+滚轮 / Ctrl++ / Ctrl+- 原生缩放

    let window = configure(builder, app_paths).build()?;
    platform::webview::register_zoom_changed_listener(&window);
    Ok(window)
}

#[cfg(target_os = "windows")]
fn configure<'a, R, M>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
    app_paths: &AppPaths,
) -> tauri::WebviewWindowBuilder<'a, R, M>
where
    R: tauri::Runtime,
    M: tauri::Manager<R>,
{
    builder.data_directory(app_paths.webview_data_dir())
}

#[cfg(unix)]
fn configure<'a, R, M>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
    _app_paths: &AppPaths,
) -> tauri::WebviewWindowBuilder<'a, R, M>
where
    R: tauri::Runtime,
    M: tauri::Manager<R>,
{
    builder.incognito(true)
}
