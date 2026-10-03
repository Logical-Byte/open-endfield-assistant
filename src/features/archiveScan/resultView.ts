import type { ArchiveAcquisitionMethod } from '@/features/gameData/types/archiveContract';
import type { PrtsData } from '@/features/gameData/types/prts';
import type { ArchiveId, ScannedItemRecord } from './types/scannedItem';
import { deriveArchiveMatching } from './matching';

export interface ArchiveDetails {
  readonly id: ArchiveId;
  readonly title: string;
  readonly category: string;
  readonly categoryLabel: string;
  readonly acquisitionMethod: ArchiveAcquisitionMethod | null;
  readonly acquisitionLabel: string | null;
  readonly oemUrl: string;
  readonly intelUrl: string;
}

export interface ArchiveEntryView extends ArchiveDetails {
  readonly scans: readonly Readonly<ScannedItemRecord>[];
}

export interface ScannedItemView extends Readonly<ScannedItemRecord> {
  readonly categoryLabel: string;
  readonly candidates: readonly string[];
  readonly archives: readonly ArchiveDetails[];
}

export interface ArchiveScanView {
  readonly archives: readonly ArchiveEntryView[];
  readonly scans: readonly ScannedItemView[];
}

const acquisitionLabels: Record<ArchiveAcquisitionMethod, string> = {
  map: '地图拾取',
  mission: '跟随任务',
  auto: '自动解锁',
  shop: '商店购买',
  invstgt: '报告摘要',
};

/** 两种展示数据共享一次匹配计算，档案目录始终来自 ground truth。 */
export function deriveArchiveScanView(
  data: PrtsData | null,
  methodByArchiveId: ReadonlyMap<string, ArchiveAcquisitionMethod>,
  scans: readonly Readonly<ScannedItemRecord>[],
): ArchiveScanView {
  const allItems = data?.allItems ?? {};
  const matching = deriveArchiveMatching(allItems, scans);
  const titlesByCategory = new Map<string, Set<string>>();
  const archiveById = new Map<ArchiveId, ArchiveEntryView>();
  function categoryLabel(page: string, category: string): string {
    return `${data?.PrtsPage[page]?.name ?? page} · ${data?.PrtsCategory[category]?.name ?? category}`;
  }
  for (const [key, archive] of Object.entries(allItems)) {
    const id = key as ArchiveId;
    const method = methodByArchiveId.get(id) ?? null;
    const titles = titlesByCategory.get(archive.categoryId) ?? new Set<string>();
    titles.add(archive.title);
    titlesByCategory.set(archive.categoryId, titles);
    archiveById.set(id, {
      id,
      title: archive.title,
      category: archive.categoryId,
      categoryLabel: categoryLabel(archive.type, archive.categoryId),
      acquisitionMethod: method,
      acquisitionLabel: method === null ? null : acquisitionLabels[method],
      oemUrl: `https://oem.re/?type=${encodeURIComponent(id)}`,
      intelUrl: `https://opendfieldmap.org/intel/?type=${encodeURIComponent(id)}`,
      scans: matching.scansByArchiveId.get(id) ?? [],
    });
  }
  return {
    archives: [...archiveById.values()],
    scans: scans.map((scan): ScannedItemView => ({
      ...scan,
      categoryLabel: categoryLabel(scan.foundInCategory, scan.foundInSubCategory),
      candidates: [...(titlesByCategory.get(scan.foundInSubCategory) ?? [])],
      archives: (matching.archiveIdsByScan.get(scan) ?? []).map((id): ArchiveDetails =>
        archiveById.get(id)!,
      ),
    })),
  };
}
