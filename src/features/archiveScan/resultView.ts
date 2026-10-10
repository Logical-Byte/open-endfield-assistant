import { type MessageKey } from '@/shared/i18n';
import type { AcquisitionMethod, ArchiveId, Category, Page } from '@/shared/types/archive';
import type { ArchiveCatalog } from '@/features/gameData/archiveCatalog';
import type { ScannedItemRecord } from './types/scannedItem';
import { deriveArchiveMatching } from './matching';

export interface ArchiveDetails {
  readonly id: ArchiveId;
  readonly title: string;
  readonly category: Category;
  readonly categoryLabel: string;
  readonly acquisitionMethod: AcquisitionMethod;
  readonly acquisitionLabel: MessageKey;
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

const acquisitionLabels: Record<AcquisitionMethod, MessageKey> = {
  map: 'scan.acquisition.map',
  mission: 'scan.acquisition.mission',
  auto: 'scan.acquisition.auto',
  shop: 'scan.acquisition.shop',
  invstgt: 'scan.acquisition.invstgt',
};

/** 两种展示数据共享一次匹配计算，档案目录始终来自真实目录。 */
export function deriveArchiveScanView(
  data: ArchiveCatalog | null,
  scans: readonly Readonly<ScannedItemRecord>[],
): ArchiveScanView {
  const archives = data?.catalog.archives ?? [];
  const matching = deriveArchiveMatching(archives, scans);
  const archiveById = new Map<ArchiveId, ArchiveEntryView>();
  function categoryLabel(page: Page, category: Category): string {
    return `${data?.page(page)?.name ?? page}/${data?.category(category)?.name ?? category}`;
  }
  for (const archive of archives) {
    const category = data!.category(archive.category)!;
    archiveById.set(archive.id, {
      id: archive.id,
      title: archive.title,
      category: archive.category,
      categoryLabel: categoryLabel(category.page, archive.category),
      acquisitionMethod: archive.acquisitionMethod,
      acquisitionLabel: acquisitionLabels[archive.acquisitionMethod],
      oemUrl: `https://oem.re/?type=${encodeURIComponent(archive.id)}`,
      intelUrl: `https://opendfieldmap.org/intel/?type=${encodeURIComponent(archive.id)}`,
      scans: matching.scansByArchiveId.get(archive.id) ?? [],
    });
  }
  return {
    archives: [...archiveById.values()],
    scans: scans.map((scan): ScannedItemView => ({
      ...scan,
      categoryLabel: categoryLabel(scan.foundInPage, scan.foundInCategory),
      candidates: data?.titles(scan.foundInCategory) ?? [],
      archives: (matching.archiveIdsByScan.get(scan) ?? []).map((id): ArchiveDetails =>
        archiveById.get(id)!,
      ),
    })),
  };
}
