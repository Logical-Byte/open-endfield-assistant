//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { ScannedItem } from './types/scannedItem';
import type { RunEvent } from '@/features/automation/types';
import { listen, type Event } from '@tauri-apps/api/event';

/**
 * 监听后端扫描结果事件（扫描进度中每识别一份档案触发一次）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export async function onScannedItem(
  cb: (result: RunEvent<ScannedItem>) => void,
): Promise<() => void> {
  return await listen<RunEvent<ScannedItem>>(
    'archive-item-scanned',
    (event: Event<RunEvent<ScannedItem>>) => cb(event.payload),
  );
}
