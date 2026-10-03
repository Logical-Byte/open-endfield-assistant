import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ArchiveId, ScannedItem } from './types/scannedItem';

export interface ArchiveMatching<T extends ScannedItem> {
  readonly scansByArchiveId: ReadonlyMap<ArchiveId, readonly T[]>;
  // 以输入记录对象为键，让导出逻辑复用匹配规则时无需依赖前端扫描 ID。
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
    // 同一条扫描可能命中多个同名 ID，在共享分组中只计一次证据。
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
