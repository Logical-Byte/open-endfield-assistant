import type { ScanResult } from '@/features/archiveScan/types/scanResult';
import { onAutomationStatus } from '@/features/automation/ipc';
import { onScanResult } from './ipc';
import type * as Automation from '@/features/automation/types';
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

/** 最近一次档案扫描失败的原因；后端只发送一次，不保存终态。 */
export const scanError = ref<string | null>(null);

export function handleArchiveScanStatus(status: Automation.Status): void {
  if (status.state === 'running' && status.taskKind === 'archiveScan') {
    scanError.value = null;
  }
}

export function handleArchiveScanRunFinished(finished: Automation.RunFinished): void {
  if (finished.taskKind === 'archiveScan' && finished.outcome.status === 'failed') {
    scanError.value = finished.outcome.error;
  }
}
