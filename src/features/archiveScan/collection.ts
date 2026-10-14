import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItemRecord } from '@/features/archiveScan/types/scannedItem';
import { deriveArchiveMatching } from './matching';

export interface ArchiveCollection {
  collectedIds: ArchiveId[];
  notCollectedIds: ArchiveId[];
}

/**
 * 根据扫描结果计算档案收集状态。
 *
 * 已收集为所有有效匹配指向的档案 ID 集合，候选不构成收集证据。
 * 返回的 ID 保持档案全集的展示顺序。
 */
export function deriveArchiveCollection(
  archives: readonly ArchiveEntry[],
  scannedItems: readonly Pick<ScannedItemRecord, 'matching' | 'manualChoice'>[],
): ArchiveCollection {
  const matching = deriveArchiveMatching(archives, scannedItems);
  const allIds = archives.map((archive): ArchiveId => archive.id);
  return {
    collectedIds: allIds.filter(
      (id: ArchiveId): boolean => (matching.scansByArchiveId.get(id)?.length ?? 0) > 0,
    ),
    notCollectedIds: allIds.filter(
      (id: ArchiveId): boolean => (matching.scansByArchiveId.get(id)?.length ?? 0) === 0,
    ),
  };
}
