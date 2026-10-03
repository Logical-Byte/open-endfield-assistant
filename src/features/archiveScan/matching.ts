import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ScannedItem } from './types/scannedItem';

export interface ArchiveMatching<T extends ScannedItem> {
  matchedByArchiveId: Record<string, T | null>;
  unmatchedItems: T[];
}

/** 同小分类、同标题共享最早的成功记录。保留输入记录类型，供卡片读取前端 ID。 */
export function deriveArchiveMatching<T extends ScannedItem>(
  allItems: Record<string, PrtsAllItem>,
  scannedItems: readonly T[],
): ArchiveMatching<T> {
  const firstMatchByGroup: Map<string, T> = new Map();
  const unmatchedItems: T[] = [];
  for (const scannedItem of scannedItems) {
    if (scannedItem.status !== 'success') {
      unmatchedItems.push(scannedItem);
      continue;
    }
    for (const id of scannedItem.correctedMatchItemIds) {
      const archive: PrtsAllItem | undefined = allItems[id];
      if (archive === undefined) continue;
      const key: string = JSON.stringify([archive.categoryId, archive.title]);
      if (!firstMatchByGroup.has(key)) firstMatchByGroup.set(key, scannedItem);
    }
  }

  const matchedByArchiveId: Record<string, T | null> = {};
  for (const [id, archive] of Object.entries(allItems)) {
    const key: string = JSON.stringify([archive.categoryId, archive.title]);
    matchedByArchiveId[id] = firstMatchByGroup.get(key) ?? null;
  }
  return { matchedByArchiveId, unmatchedItems };
}
