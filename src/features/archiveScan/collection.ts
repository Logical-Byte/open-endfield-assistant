import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ScannedItem } from '@/features/archiveScan/types/scannedItem';
import { deriveArchiveMatching, type ArchiveMatching } from './matching';

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
  scannedItems: readonly ScannedItem[],
): ArchiveCollection {
  const matching: ArchiveMatching<ScannedItem> = deriveArchiveMatching(allItems, scannedItems);
  const allIds: string[] = Object.keys(allItems);
  return {
    collectedIds: allIds.filter((id: string): boolean => matching.matchedByArchiveId[id] !== null),
    notCollectedIds: allIds.filter(
      (id: string): boolean => matching.matchedByArchiveId[id] === null,
    ),
  };
}
