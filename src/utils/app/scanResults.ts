import type { ScanResult } from '@/types/scanResult';
import { onScanResult } from '@/utils/tauri';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加，按序号排序） */
export const scanResults = ref<ScanResult[]>([]);

/** 清空扫描结果列表。 */
export function clearScanResults(): void {
  scanResults.value = [];
}

export async function initScanResults() {
  await onScanResult((result) => {
    scanResults.value.push(result);
  });
}
