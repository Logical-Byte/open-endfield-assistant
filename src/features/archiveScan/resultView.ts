import { type MessageKey } from '@/shared/i18n';
import type { AcquisitionMethod, ArchiveId, Category, Page } from '@/shared/types/archive';
import type { ArchiveCatalog } from '@/features/gameData/archiveCatalog';
import type { ScannedItemRecord } from './types/scannedItem';
import { deriveArchiveMatching } from './matching';

export interface ArchiveDetails {
  readonly id: ArchiveId;
  readonly title: string;
  readonly contentPreview?: string;
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
  readonly options: readonly ArchiveDetails[];
  readonly recommendations: readonly ArchiveDetails[];
  readonly archive: ArchiveDetails | null;
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
    return `${data?.page(page)?.name['zh-CN'] ?? page}/${data?.category(category)?.name['zh-CN'] ?? category}`;
  }
  for (const archive of archives) {
    const category = data!.category(archive.category)!;
    archiveById.set(archive.id, {
      id: archive.id,
      title: archive.title['zh-CN'],
      contentPreview: archive.contentPreview?.['zh-CN'],
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
    scans: scans.map((scan): ScannedItemView => {
      const ids =
        scan.matching.kind === 'exact' ? [scan.matching.archiveId] : scan.matching.archiveIds;
      const id = matching.archiveIdByScan.get(scan);
      return {
        ...scan,
        categoryLabel: categoryLabel(scan.foundInPage, scan.foundInCategory),
        options: [...archiveById.values()].filter(
          (archive): boolean => archive.category === scan.foundInCategory,
        ),
        recommendations: ids.map((id): ArchiveDetails => archiveById.get(id)!),
        archive: id ? archiveById.get(id)! : null,
      };
    }),
  };
}
