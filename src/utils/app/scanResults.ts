import type { ScanResult } from '@/types/scanResult';
import { onAppStatus, onScanResult } from '@/utils/tauri';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加） */
export const scanResults = ref<ScanResult[]>([]);

/** 清空扫描结果列表。 */
export function clearScanResults(): void {
  scanResults.value = [];
}

export async function initScanResults() {
  // 每次开始新扫描（后端 running 置位，含热键触发）时清空上次任务的结果
  await onAppStatus((status) => {
    if (status.running) {
      clearScanResults();
    }
  });
  await onScanResult((result) => {
    scanResults.value.push(result);
  });
}
