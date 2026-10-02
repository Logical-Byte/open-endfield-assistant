import type { ScannedItem } from '@/features/archiveScan/types/scannedItem';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import { onScannedItem } from './ipc';
import { whenever } from '@vueuse/core';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加） */
export const scannedItems = ref<ScannedItem[]>([]);

/** 最近一次档案扫描失败的原因，新扫描启动时清除。 */
export const scanError = ref<string | null>(null);

/** 清空扫描结果列表。 */
export function clearScannedItems(): void {
  scannedItems.value = [];
}

export async function initScannedItems(): Promise<void> {
  const task = useAutomationTask('archiveScan');
  whenever(
    () => task.phase.value === 'running',
    () => {
      clearScannedItems();
      scanError.value = null;
    },
    { flush: 'sync' },
  );
  whenever(
    task.outcome,
    (outcome) => {
      if (outcome.status === 'failed') scanError.value = outcome.error;
    },
    { flush: 'sync' },
  );
  await onScannedItem((result) => {
    scannedItems.value.push(result);
  });
}
