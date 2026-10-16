//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { ScreenshotFormat } from '@/features/monitor/types/screenshot';
import { invoke } from '@tauri-apps/api/core';

/**
 * 截取游戏窗口画面：按指定尺寸缩放并编码为指定格式，返回 base64 图片数据
 * （不含 data URL 前缀，调用方自行拼接）。帧率控制等轮询逻辑由前端负责。
 */
export async function screenshot(
  width: number,
  height: number,
  format: ScreenshotFormat,
): Promise<string> {
  return await invoke<string>('screenshot', { width, height, format });
}
