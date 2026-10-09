import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItem } from './types/scannedItem';

type ScanEvidence = Pick<ScannedItem, 'status'> & {
  readonly correctedMatchItemIds: readonly ArchiveId[];
};

export interface ArchiveMatching<T extends ScanEvidence> {
  readonly scansByArchiveId: ReadonlyMap<ArchiveId, readonly T[]>;
  // 以输入记录对象为键，导出复用匹配规则时无需依赖前端扫描 ID。
  readonly archiveIdsByScan: ReadonlyMap<T, readonly ArchiveId[]>;
}

/** 同分类、同标题共享成功扫描证据，保留全部记录及双向关系。 */
export function deriveArchiveMatching<T extends ScanEvidence>(
  archives: readonly ArchiveEntry[],
  scannedItems: readonly T[],
): ArchiveMatching<T> {
  const byId = new Map(archives.map((archive) => [archive.id, archive]));
  const scansByGroup = new Map<string, T[]>();
  for (const scan of scannedItems) {
    if (scan.status !== 'success') continue;
    const groups = new Set<string>();
    for (const id of scan.correctedMatchItemIds) {
      const archive = byId.get(id);
      if (archive) groups.add(JSON.stringify([archive.category, archive.title]));
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
  for (const archive of archives) {
    const scans = scansByGroup.get(JSON.stringify([archive.category, archive.title])) ?? [];
    scansByArchiveId.set(archive.id, scans);
    for (const scan of scans) archiveIdsByScan.get(scan)!.push(archive.id);
  }
  return { scansByArchiveId, archiveIdsByScan };
}
