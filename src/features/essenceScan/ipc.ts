import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { RunEvent } from '@/features/automation/types';
import type { EssenceCatalog, ScannedItem } from './types';

export async function getEssenceCatalog(): Promise<EssenceCatalog> {
  return await invoke('get_essence_catalog');
}

export async function onScannedItem(
  callback: (event: RunEvent<ScannedItem>) => void,
): Promise<() => void> {
  return await listen<RunEvent<ScannedItem>>('essence-item-scanned', (event) =>
    callback(event.payload),
  );
}
