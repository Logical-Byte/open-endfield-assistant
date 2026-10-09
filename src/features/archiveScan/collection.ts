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
 * 同一小分类下的同标题档案中只要有一项成功命中，该组档案都视为已收集。
 * 返回的 ID 保持档案全集的展示顺序。
 */
export function deriveArchiveCollection(
  archives: readonly ArchiveEntry[],
  scannedItems: readonly Pick<ScannedItemRecord, 'status' | 'correctedMatchItemIds'>[],
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
