import { t } from '@/shared/i18n';
import type { Category } from '@/shared/types/archive';
import { computed, ref, watch, type Ref, type ComputedRef } from 'vue';
import { archiveCatalog, type ArchiveCatalog } from '@/features/gameData/archiveCatalog';
import {
  scannedItems,
  correctScannedItem,
  clearScannedItems,
  restoreScannedItemCorrection,
} from './scannedItems';
import { deriveArchiveScanView, type ArchiveEntryView, type ScannedItemView } from './resultView';
import type { ScannedItemId, ScannedItemRecord } from './types/scannedItem';

/** 页面只观察这一数据源，写入通过指定操作完成。也可供独立预览使用。 */
export interface ArchiveScanSource {
  readonly data: Readonly<Ref<ArchiveCatalog | null>>;
  readonly scans: Readonly<Ref<readonly Readonly<ScannedItemRecord>[]>>;
  correct: (id: ScannedItemId, title: string) => void;
  clear: () => void;
  restore: (previous: Readonly<ScannedItemRecord>) => void;
}
const liveSource: ArchiveScanSource = {
  data: archiveCatalog,
  scans: scannedItems,
  correct: correctScannedItem,
  clear: clearScannedItems,
  restore: restoreScannedItemCorrection,
};
export type MatchingFilter = 'unmatched' | 'matched' | 'all';
function acceptsMatch(filter: MatchingFilter, matched: boolean): boolean {
  return filter === 'all' || (filter === 'matched' ? matched : !matched);
}
// Reka Select 将空字符串用于清除选择，选项本身必须有非空值。
const ALL_CATEGORIES = 'all';
type CategoryFilter = Category | typeof ALL_CATEGORIES;
interface CategoryOption {
  label: string;
  value: CategoryFilter;
}
export interface ScanCorrection {
  readonly key: number;
  readonly before: Readonly<ScannedItemRecord>;
  readonly after: Readonly<ScannedItemRecord>;
  undone: boolean;
}
interface ArchiveScanResultsState {
  archives: ComputedRef<readonly ArchiveEntryView[]>;
  scans: ComputedRef<readonly ScannedItemView[]>;
  matchedArchives: ComputedRef<number>;
  matchedScans: ComputedRef<number>;
  categories: ComputedRef<CategoryOption[]>;
  visibleArchives: ComputedRef<ArchiveEntryView[]>;
  visibleScans: ComputedRef<ScannedItemView[]>;
  archiveFilter: Ref<MatchingFilter>;
  scanFilter: Ref<MatchingFilter>;
  archiveSearch: Ref<string>;
  scanSearch: Ref<string>;
  archiveCategory: Ref<CategoryFilter>;
  scanCategory: Ref<CategoryFilter>;
  mapOnly: Ref<boolean>;
  corrections: Ref<ScanCorrection[]>;
  correct: (id: ScannedItemId, title: string) => ScanCorrection;
  undo: (correction: ScanCorrection) => void;
  clear: () => void;
  revealScan: () => void;
  revealArchive: () => void;
}
/**
 * 每次调用持有独立的筛选和编辑撤销状态，默认连接共享的真实扫描数据。
 * 两栏数据从同一匹配结果派生，筛选只影响显示，不影响关联与导出。
 * 滚动和临时高亮由页面负责。
 */
export function useArchiveScanResults(
  source: ArchiveScanSource = liveSource,
): ArchiveScanResultsState {
  const view = computed((): ReturnType<typeof deriveArchiveScanView> =>
    deriveArchiveScanView(source.data.value, source.scans.value),
  );
  const archiveFilter: Ref<MatchingFilter> = ref<MatchingFilter>('all');
  const scanFilter: Ref<MatchingFilter> = ref<MatchingFilter>('unmatched');
  const archiveSearch: Ref<string> = ref('');
  const scanSearch: Ref<string> = ref('');
  const archiveCategory: Ref<CategoryFilter> = ref<CategoryFilter>(ALL_CATEGORIES);
  const scanCategory: Ref<CategoryFilter> = ref<CategoryFilter>(ALL_CATEGORIES);
  const mapOnly: Ref<boolean> = ref(false);
  const corrections: Ref<ScanCorrection[]> = ref([]);
  let nextCorrection = 0;
  const archives = computed((): readonly ArchiveEntryView[] => view.value.archives);
  const scans = computed((): readonly ScannedItemView[] => view.value.scans);
  const matchedArchives = computed(
    (): number =>
      archives.value.filter((a: ArchiveEntryView): boolean => a.scans.length > 0).length,
  );
  const matchedScans = computed(
    (): number => scans.value.filter((s: ScannedItemView): boolean => s.archives.length > 0).length,
  );
  const categories = computed((): CategoryOption[] => [
    { label: t('scan.allCategories'), value: ALL_CATEGORIES },
    ...(source.data.value?.catalog.categories ?? []).map((category): CategoryOption => ({
      label: category.name['zh-CN'],
      value: category.id,
    })),
  ]);
  const visibleArchives = computed((): ArchiveEntryView[] =>
    archives.value.filter(
      (a: ArchiveEntryView): boolean =>
        acceptsMatch(archiveFilter.value, a.scans.length > 0) &&
        (!mapOnly.value || a.acquisitionMethod === 'map') &&
        (archiveCategory.value === ALL_CATEGORIES || a.category === archiveCategory.value) &&
        a.title.includes(archiveSearch.value),
    ),
  );
  const visibleScans = computed((): ScannedItemView[] =>
    scans.value.filter(
      (s: ScannedItemView): boolean =>
        acceptsMatch(scanFilter.value, s.archives.length > 0) &&
        (scanCategory.value === ALL_CATEGORIES || s.foundInCategory === scanCategory.value) &&
        [s.ocrResult, s.correctedTitle ?? ''].some((title: string): boolean =>
          title.includes(scanSearch.value),
        ),
    ),
  );
  // 扫描记录消失时同时作废撤销历史，避免旧通知改动新一轮扫描。
  watch(
    source.scans,
    (items: readonly Readonly<ScannedItemRecord>[]): void => {
      const ids = new Set(
        items.map((item: Readonly<ScannedItemRecord>): ScannedItemId => item.scannedItemId),
      );
      corrections.value = corrections.value.filter((edit: ScanCorrection): boolean =>
        ids.has(edit.before.scannedItemId),
      );
    },
    { flush: 'sync' },
  );
  function correct(id: ScannedItemId, title: string): ScanCorrection {
    const before = source.scans.value.find(
      (item: Readonly<ScannedItemRecord>): boolean => item.scannedItemId === id,
    )!;
    source.correct(id, title);
    const after = source.scans.value.find(
      (item: Readonly<ScannedItemRecord>): boolean => item.scannedItemId === id,
    )!;
    const correction: ScanCorrection = { key: ++nextCorrection, before, after, undone: false };
    corrections.value.push(correction);
    return correction;
  }
  function undo(correction: ScanCorrection): void {
    const history = corrections.value.filter(
      (edit: ScanCorrection): boolean =>
        edit.before.scannedItemId === correction.before.scannedItemId,
    );
    const edit = history.find((entry: ScanCorrection): boolean => entry.key === correction.key);
    if (!edit || edit.undone) return;
    edit.undone = true;
    // 撤销较早的编辑时保留后续编辑。全部撤销才恢复第一次编辑前的结果。
    const remaining = history.filter((entry: ScanCorrection): boolean => !entry.undone);
    source.restore(remaining[remaining.length - 1]?.after ?? history[0]!.before);
  }
  function clear(): void {
    source.clear();
    corrections.value = [];
  }
  // 定位前解除目标栏筛选，避免关联记录被当前条件隐藏。
  function revealScan(): void {
    scanFilter.value = 'all';
    scanSearch.value = '';
    scanCategory.value = ALL_CATEGORIES;
  }
  function revealArchive(): void {
    archiveFilter.value = 'all';
    archiveSearch.value = '';
    archiveCategory.value = ALL_CATEGORIES;
    mapOnly.value = false;
  }
  return {
    archives,
    scans,
    matchedArchives,
    matchedScans,
    categories,
    visibleArchives,
    visibleScans,
    archiveFilter,
    scanFilter,
    archiveSearch,
    scanSearch,
    archiveCategory,
    scanCategory,
    mapOnly,
    corrections,
    correct,
    undo,
    clear,
    revealScan,
    revealArchive,
  };
}
