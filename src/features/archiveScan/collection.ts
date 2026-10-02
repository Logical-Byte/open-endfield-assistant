import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ScanResult } from '@/features/archiveScan/types/scanResult';

export interface ArchiveCollection {
  collectedIds: string[];
  notCollectedIds: string[];
}

/**
 * 根据扫描结果计算档案收集状态。
 *
 * 同一小分类下的同标题档案中只要有一项成功命中，该组档案都视为已收集。
 * 返回的 ID 保持档案全集的展示顺序。
 */
export function deriveArchiveCollection(
  allItems: Record<string, PrtsAllItem>,
  scanResults: readonly ScanResult[],
): ArchiveCollection {
  const collected = new Set<string>();
  for (const result of scanResults) {
    if (result.status === 'success') {
      for (const id of result.itemIds) {
        collected.add(id);
      }
    }
  }

  const idsByCategoryAndTitle = new Map<string, string[]>();
  for (const item of Object.values(allItems)) {
    const key = JSON.stringify([item.categoryId, item.title]);
    const ids = idsByCategoryAndTitle.get(key) ?? [];
    ids.push(item.id);
    idsByCategoryAndTitle.set(key, ids);
  }
  for (const ids of idsByCategoryAndTitle.values()) {
    if (ids.some((id) => collected.has(id))) {
      for (const id of ids) {
        collected.add(id);
      }
    }
  }

  const allIds = Object.keys(allItems);
  return {
    collectedIds: allIds.filter((id) => collected.has(id)),
    notCollectedIds: allIds.filter((id) => !collected.has(id)),
  };
}
