//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { PrtsData } from '@/features/gameData/types/prts';
import type { ArchiveContract } from '@/features/gameData/types/archiveContract';
import { invoke } from '@tauri-apps/api/core';

/** 获取 prts.json 完整数据（分类中文名映射 / 自动补全候选）。 */
export async function getPrtsData(): Promise<PrtsData> {
  return await invoke('get_prts_data');
}

/** 获取档案获取契约数据（按档案 id 查询获取方式）。 */
export async function getArchiveContract(): Promise<ArchiveContract> {
  return await invoke('get_archive_contract');
}
