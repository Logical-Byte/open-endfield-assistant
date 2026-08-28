//! WebView2 原生缩放接口。

use std::sync::mpsc;

use anyhow::{Context, Result, anyhow};
use tauri::{Emitter, Manager};
use tracing::warn;
use webview2_com::{
    Microsoft::Web::WebView2::Win32::ICoreWebView2Controller, ZoomFactorChangedEventHandler,
};
use windows::core::IUnknown;

/// 读取 WebView2 当前缩放因子。
pub(in crate::windows_ops) fn get_zoom(window: tauri::WebviewWindow) -> Result<f64> {
    let (tx, rx) = mpsc::channel();
    window
        .with_webview(move |platform_webview| {
            let controller = platform_webview.controller();
            let mut factor = 0.0f64;
            let zoom = unsafe { controller.ZoomFactor(&mut factor) }
                .ok()
                .map(|_| factor);
            let _ = tx.send(zoom);
        })
        .context("获取 WebView2 controller 失败")?;
    rx.recv()
        .context("接收 WebView2 缩放因子失败")?
        .ok_or_else(|| anyhow!("读取 WebView2 缩放因子失败"))
}

/// 注册 WebView2 原生缩放（`ZoomFactor`）变化监听。
///
/// 用户通过 `Ctrl+滚轮` / `Ctrl+加减` 缩放时，WebView2 内部会修改 `ZoomFactor` 并触发
/// `ZoomFactorChanged` 事件。这里把新值 emit 给前端（`webview-zoom-changed`），
/// 让设置页的缩放滑块与快捷键缩放保持同步。
pub(in crate::windows_ops) fn register_zoom_changed_listener(window: &tauri::WebviewWindow) {
    let app_handle = window.app_handle().clone();

    let result = window.with_webview(move |platform_webview| {
        let controller = platform_webview.controller();
        let handler = ZoomFactorChangedEventHandler::create(Box::new(
            move |sender: Option<ICoreWebView2Controller>, _args: Option<IUnknown>| {
                let Some(controller) = sender else {
                    return Ok(());
                };
                let mut factor = 0.0f64;
                unsafe { controller.ZoomFactor(&mut factor) }?;
                let _ = app_handle.emit("webview-zoom-changed", factor);
                Ok(())
            },
        ));

        let mut token = 0i64;
        if let Err(e) = unsafe { controller.add_ZoomFactorChanged(&handler, &mut token) } {
            warn!("注册 ZoomFactorChanged 监听失败: {e}");
        }
    });

    if let Err(e) = result {
        warn!("获取 WebView2 controller 失败: {e:#}");
    }
}
