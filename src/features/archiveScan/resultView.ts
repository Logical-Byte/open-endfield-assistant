import type { ArchiveAcquisitionMethod } from '@/features/gameData/types/archiveContract';
import type { PrtsAllItem, PrtsData } from '@/features/gameData/types/prts';
import type { ScannedItemRecord } from './scannedItems';
import { deriveArchiveMatching, type ArchiveMatching } from './matching';
import {
  ArchiveScanCardStatus,
  type ArchiveScanResultCardProps,
} from './types/archiveScanResultCard';

export interface ArchiveScanSummary {
  notSuccessCount: number;
  notCollectedCount: number;
  collectedCount: number;
}

export interface ArchiveScanView {
  cards: ArchiveScanResultCardProps[];
  summary: ArchiveScanSummary;
}

export interface ArchiveScanFilters {
  hideCollected: boolean;
  hideNotObtainableInOverworld: boolean;
}

interface CardDetails {
  status: ArchiveScanCardStatus;
  category: string;
  subCategory: string;
  imageUrl: string | null;
  title: string;
  archiveId: string | null;
  scannedItemId: number | null;
}

const ACQUISITION_METHOD_LABELS: Record<ArchiveAcquisitionMethod, string> = {
  map: '地图拾取',
  mission: '跟随任务',
  auto: '自动解锁',
  shop: '商店购买',
  invstgt: '报告摘要',
};

function archiveUrl(base: string, archiveId: string | null): string | null {
  if (archiveId === null) return null;
  const url: URL = new URL(base);
  url.searchParams.set('type', archiveId);
  return url.toString();
}

/** 先展示待纠错记录，再按目录顺序展示档案。统计不受可见性筛选影响。 */
export function deriveArchiveScanView(
  data: PrtsData | null,
  methodByArchiveId: ReadonlyMap<string, ArchiveAcquisitionMethod>,
  scannedItems: readonly Readonly<ScannedItemRecord>[],
): ArchiveScanView {
  const allItems: Record<string, PrtsAllItem> = data?.allItems ?? {};
  const matching: ArchiveMatching<Readonly<ScannedItemRecord>> = deriveArchiveMatching(
    allItems,
    scannedItems,
  );
  const titlesByCategory: Map<string, string[]> = new Map();
  for (const archive of Object.values(allItems)) {
    const titles: string[] = titlesByCategory.get(archive.categoryId) ?? [];
    titles.push(archive.title);
    titlesByCategory.set(archive.categoryId, titles);
  }

  function makeCard(details: CardDetails): ArchiveScanResultCardProps {
    const { category, subCategory, ...display }: CardDetails = details;
    const method: ArchiveAcquisitionMethod | null =
      details.archiveId === null ? null : (methodByArchiveId.get(details.archiveId) ?? null);
    return {
      ...display,
      categoryLabel:
        category && subCategory
          ? `${data?.PrtsPage[category]?.name ?? category} − ${data?.PrtsCategory[subCategory]?.name ?? subCategory}`
          : null,
      candidates: titlesByCategory.get(subCategory) ?? [],
      acquisitionMethod: method,
      acquisitionLabel:
        method !== null && method !== 'map' ? ACQUISITION_METHOD_LABELS[method] : null,
      oemUrl: archiveUrl('https://oem.re/', details.archiveId),
      intelUrl: archiveUrl('https://opendfieldmap.org/intel/', details.archiveId),
    };
  }

  const cards: ArchiveScanResultCardProps[] = [];
  for (const item of matching.unmatchedItems) {
    cards.push(
      makeCard({
        status:
          item.status === 'failed'
            ? ArchiveScanCardStatus.OcrFailed
            : ArchiveScanCardStatus.Unrecognized,
        category: item.foundInCategory,
        subCategory: item.foundInSubCategory,
        imageUrl: item.image,
        title: item.correctedTitle ?? item.ocrResult,
        archiveId: item.correctedMatchItemIds[0] ?? null,
        scannedItemId: item.scannedItemId,
      }),
    );
  }

  let collectedCount: number = 0;
  for (const [id, archive] of Object.entries(allItems)) {
    const matched: Readonly<ScannedItemRecord> | null = matching.matchedByArchiveId[id]!;
    if (matched !== null) collectedCount++;
    cards.push(
      makeCard({
        status: matched === null ? ArchiveScanCardStatus.NotMatched : ArchiveScanCardStatus.Matched,
        category: archive.type,
        subCategory: archive.categoryId,
        imageUrl: matched?.image ?? null,
        title: matched?.correctedTitle ?? archive.title,
        archiveId: id,
        scannedItemId: matched?.scannedItemId ?? null,
      }),
    );
  }
  return {
    cards,
    summary: {
      notSuccessCount: matching.unmatchedItems.length,
      notCollectedCount: Object.keys(allItems).length - collectedCount,
      collectedCount,
    },
  };
}

/** 可见性筛选只针对目录档案，待纠错记录始终保留。 */
export function filterArchiveScanCards(
  cards: readonly ArchiveScanResultCardProps[],
  filters: ArchiveScanFilters,
): ArchiveScanResultCardProps[] {
  return cards.filter((card: ArchiveScanResultCardProps): boolean => {
    if (
      card.status === ArchiveScanCardStatus.Unrecognized ||
      card.status === ArchiveScanCardStatus.OcrFailed
    )
      return true;
    if (filters.hideCollected && card.status === ArchiveScanCardStatus.Matched) return false;
    return !filters.hideNotObtainableInOverworld || card.acquisitionMethod === 'map';
  });
}
