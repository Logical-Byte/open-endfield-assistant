//! 共享扫描结果状态（模块级单例 ref，多个组件共享）。
import type { ScanResult } from '@/lib/tauri';
import { onScanResult } from '@/lib/tauri';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加，按序号排序） */
export const scanResults = ref<ScanResult[]>([]);

let initialized = false;

/** 初始化：订阅后端扫描结果事件（幂等，全局只需调用一次）。 */
export async function initScanResults(): Promise<void> {
  if (initialized) {
    return;
  }
  initialized = true;
  await onScanResult((result) => {
    scanResults.value.push(result);
    // 按序号排序，防止事件乱序导致展示错乱
    scanResults.value.sort((a, b) => a.index - b.index);
  });
}

/** 清空扫描结果列表。 */
export function clearScanResults(): void {
  scanResults.value = [];
}
