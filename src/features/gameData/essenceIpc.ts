import type { Catalog } from '@/shared/types/essence';
import { invoke } from '@tauri-apps/api/core';

/** 获取经 Rust 校验的基质词条和武器目录。 */
export async function getEssenceCatalog(): Promise<Catalog> {
  return await invoke('get_essence_catalog');
}
