//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** 读取 WebView2 当前缩放因子（用于初始化缩放滑块）。 */
export async function getWebviewZoom(): Promise<number> {
  return await invoke<number>('get_webview_zoom');
}

export async function onWebviewZoomChanged(cb: (zoom: number) => void): Promise<() => void> {
  return await listen<number>('webview-zoom-changed', (event) => cb(event.payload));
}
