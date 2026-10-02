import type { ScanResult } from '@/features/archiveScan/types/scanResult';
import { automationStatus } from '@/features/automation/state';
import { onScanResult } from './ipc';
import { ref, watch } from 'vue';

/** 扫描结果列表（随扫描进度实时追加） */
export const scanResults = ref<ScanResult[]>([]);

/** 最近一次档案扫描失败的原因，新扫描启动时清除。 */
export const scanError = ref<string | null>(null);

/** 清空扫描结果列表。 */
export function clearScanResults(): void {
  scanResults.value = [];
}

export async function initScanResults(): Promise<void> {
  // 每次开始新档案扫描（含热键触发）时清空上次任务的结果。
  watch(
    automationStatus,
    (status) => {
      if (status.state === 'running' && status.taskKind === 'archiveScan') {
        clearScanResults();
        scanError.value = null;
      }
      if (
        status.state === 'idle' &&
        status.lastRun?.taskKind === 'archiveScan' &&
        status.lastRun.outcome.status === 'failed'
      ) {
        scanError.value = status.lastRun.outcome.error;
      }
    },
    // 启动时同步清空旧结果，避免延后的 watcher 清掉刚追加的新结果。
    { flush: 'sync' },
  );
  await onScanResult((result) => {
    scanResults.value.push(result);
  });
}
