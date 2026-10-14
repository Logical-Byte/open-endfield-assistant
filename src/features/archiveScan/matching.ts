import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItemRecord } from './types/scannedItem';

type ScanEvidence = Pick<ScannedItemRecord, 'matching' | 'manualChoice'>;

/** 人工决定优先，推荐候选永远不直接成为匹配。 */
export function matchedArchiveId(scan: ScanEvidence): ArchiveId | null {
  if (scan.manualChoice.kind === 'manual') return scan.manualChoice.archiveId;
  return scan.matching.kind === 'exact' ? scan.matching.archiveId : null;
}

export interface ArchiveMatching<T extends ScanEvidence> {
  readonly scansByArchiveId: ReadonlyMap<ArchiveId, readonly T[]>;
  readonly archiveIdByScan: ReadonlyMap<T, ArchiveId | null>;
}

/** 只按具体档案 ID 关联证据，每条扫描记录最多关联一个档案。 */
export function deriveArchiveMatching<T extends ScanEvidence>(
  archives: readonly ArchiveEntry[],
  scannedItems: readonly T[],
): ArchiveMatching<T> {
  const scansByArchiveId = new Map<ArchiveId, T[]>(
    archives.map((archive): [ArchiveId, T[]] => [archive.id, []]),
  );
  const archiveIdByScan = new Map<T, ArchiveId | null>();
  for (const scan of scannedItems) {
    const id = matchedArchiveId(scan);
    const evidence = id ? scansByArchiveId.get(id) : undefined;
    archiveIdByScan.set(scan, evidence ? id : null);
    if (evidence) evidence.push(scan);
  }
  return { scansByArchiveId, archiveIdByScan };
}
