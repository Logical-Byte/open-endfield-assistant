import type { Catalog } from '@/shared/types/archive';
import { invoke } from '@tauri-apps/api/core';

/** 获取展示、人工纠错与导出使用的有序精简目录。 */
export async function getArchiveCatalog(): Promise<Catalog> {
  return await invoke('get_archive_catalog');
}
