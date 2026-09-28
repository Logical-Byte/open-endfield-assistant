import type { ScanResult } from '@/types/scanResult';
import { onAutomationStatus, onScanResult } from '@/utils/tauri';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加） */
export const scanResults = ref<ScanResult[]>([]);

/** 清空扫描结果列表。 */
export function clearScanResults(): void {
  scanResults.value = [];
}

export async function initScanResults() {
  // 每次开始新档案扫描（含热键触发）时清空上次任务的结果。
  await onAutomationStatus((status) => {
    if (status.state === 'running' && status.taskKind === 'archiveScan') {
      clearScanResults();
    }
  });
  await onScanResult((result) => {
    scanResults.value.push(result);
  });
}
