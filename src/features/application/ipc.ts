//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import { invoke } from '@tauri-apps/api/core';

/** 退出程序。 */
export async function quitApp(): Promise<void> {
  return await invoke('quit');
}
