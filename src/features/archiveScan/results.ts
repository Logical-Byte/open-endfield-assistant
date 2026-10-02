import type { ScanResult } from '@/features/archiveScan/types/scanResult';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import { onScanResult } from './ipc';
import { whenever } from '@vueuse/core';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加） */
export const scanResults = ref<ScanResult[]>([]);

/** 最近一次档案扫描失败的原因，新扫描启动时清除。 */
export const scanError = ref<string | null>(null);

/** 清空扫描结果列表。 */
export function clearScanResults(): void {
  scanResults.value = [];
}

export async function initScanResults(): Promise<void> {
  const task = useAutomationTask('archiveScan');
  whenever(
    () => task.phase.value === 'running',
    () => {
      clearScanResults();
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
  await onScanResult((result) => {
    scanResults.value.push(result);
  });
}
