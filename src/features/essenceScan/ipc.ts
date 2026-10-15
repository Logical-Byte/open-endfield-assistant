import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  EssenceScanReport,
  EssenceScanRecord,
  EssenceScanProgress,
} from '@/shared/types/essenceScan';

export async function getEssenceScanReport(): Promise<EssenceScanReport | null> {
  return await invoke('get_essence_scan_report');
}

export async function onEssenceItemScanned(
  callback: (record: EssenceScanRecord) => void,
): Promise<() => void> {
  return await listen<EssenceScanRecord>('essence-item-scanned', (event) =>
    callback(event.payload),
  );
}

export async function onEssenceScanProgress(
  callback: (progress: EssenceScanProgress) => void,
): Promise<() => void> {
  return await listen<EssenceScanProgress>('essence-scan-progress', (event) =>
    callback(event.payload),
  );
}
