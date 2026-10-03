import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ArchiveId, ScannedItem } from './types/scannedItem';

export interface ArchiveMatching<T extends ScannedItem> {
  readonly scansByArchiveId: ReadonlyMap<ArchiveId, readonly T[]>;
  readonly archiveIdsByScan: ReadonlyMap<T, readonly ArchiveId[]>;
}

/** 同小分类、同标题共享成功扫描证据，保留全部记录及双向关系。 */
export function deriveArchiveMatching<T extends ScannedItem>(
  allItems: Record<string, PrtsAllItem>,
  scannedItems: readonly T[],
): ArchiveMatching<T> {
  const scansByGroup = new Map<string, T[]>();
  for (const scan of scannedItems) {
    if (scan.status !== 'success') continue;
    const groups = new Set<string>();
    for (const id of scan.correctedMatchItemIds) {
      const archive = allItems[id];
      if (archive) groups.add(JSON.stringify([archive.categoryId, archive.title]));
    }
    for (const group of groups) {
      const scans = scansByGroup.get(group) ?? [];
      scans.push(scan);
      scansByGroup.set(group, scans);
    }
  }

  const scansByArchiveId = new Map<ArchiveId, readonly T[]>();
  const archiveIdsByScan = new Map<T, ArchiveId[]>();
  for (const scan of scannedItems) archiveIdsByScan.set(scan, []);
  for (const [key, archive] of Object.entries(allItems)) {
    const id = key as ArchiveId;
    const scans = scansByGroup.get(JSON.stringify([archive.categoryId, archive.title])) ?? [];
    scansByArchiveId.set(id, scans);
    for (const scan of scans) archiveIdsByScan.get(scan)!.push(id);
  }
  return { scansByArchiveId, archiveIdsByScan };
}
