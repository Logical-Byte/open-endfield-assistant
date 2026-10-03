import { computed, ref, watch, type Ref, type ComputedRef } from 'vue';
import { prtsData } from '@/features/gameData/prtsData';
import { methodByArchiveId } from '@/features/gameData/archiveContract';
import type { PrtsData } from '@/features/gameData/types/prts';
import type { ArchiveAcquisitionMethod } from '@/features/gameData/types/archiveContract';
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
  readonly data: Readonly<Ref<PrtsData | null>>;
  readonly methods: Readonly<Ref<ReadonlyMap<string, ArchiveAcquisitionMethod>>>;
  readonly scans: Readonly<Ref<readonly Readonly<ScannedItemRecord>[]>>;
  correct: (id: ScannedItemId, title: string) => void;
  clear: () => void;
  restore: (previous: Readonly<ScannedItemRecord>) => void;
}
const liveSource: ArchiveScanSource = {
  data: prtsData,
  methods: methodByArchiveId,
  scans: scannedItems,
  correct: correctScannedItem,
  clear: clearScannedItems,
  restore: restoreScannedItemCorrection,
};
export type MatchingFilter = 'unmatched' | 'matched' | 'all';
export function filterOptions(
  total: number,
  matched: number,
): { label: string; value: MatchingFilter; count: number }[] {
  return [
    { label: '未匹配', value: 'unmatched', count: total - matched },
    { label: '有匹配', value: 'matched', count: matched },
    { label: '全部', value: 'all', count: total },
  ];
}
function acceptsMatch(filter: MatchingFilter, matched: boolean): boolean {
  return filter === 'all' || (filter === 'matched' ? matched : !matched);
}
// Reka Select 将空字符串用于清除选择，选项本身必须有非空值。
const ALL_CATEGORIES = 'all';
interface CategoryOption {
  label: string;
  value: string;
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
  archiveCategory: Ref<string>;
  scanCategory: Ref<string>;
  mapOnly: Ref<boolean>;
  lastEdit: Ref<Readonly<ScannedItemRecord> | null>;
  correct: (id: ScannedItemId, title: string) => number;
  undo: () => void;
  clear: () => void;
  revealScan: () => void;
  revealArchive: () => void;
}
/**
 * 每次调用持有独立的筛选和单步撤销状态，默认连接共享的真实扫描数据。
 * 两栏数据从同一匹配结果派生，筛选只影响显示，不影响关联与导出。
 * 滚动和临时高亮由页面负责。
 */
export function useArchiveScanResults(
  source: ArchiveScanSource = liveSource,
): ArchiveScanResultsState {
  const view = computed((): ReturnType<typeof deriveArchiveScanView> =>
    deriveArchiveScanView(source.data.value, source.methods.value, source.scans.value),
  );
  const archiveFilter = ref<MatchingFilter>('unmatched');
  const scanFilter = ref<MatchingFilter>('unmatched');
  const archiveSearch = ref('');
  const scanSearch = ref('');
  const archiveCategory = ref(ALL_CATEGORIES);
  const scanCategory = ref(ALL_CATEGORIES);
  const mapOnly = ref(false);
  const lastEdit: Ref<Readonly<ScannedItemRecord> | null> = ref(null);
  const archives = computed((): readonly ArchiveEntryView[] => view.value.archives);
  const scans = computed((): readonly ScannedItemView[] => view.value.scans);
  const matchedArchives = computed(
    (): number => archives.value.filter((a): boolean => a.scans.length > 0).length,
  );
  const matchedScans = computed(
    (): number => scans.value.filter((s): boolean => s.archives.length > 0).length,
  );
  const categories = computed((): CategoryOption[] => [
    { label: '全部分类', value: ALL_CATEGORIES },
    ...Object.values(source.data.value?.PrtsCategory ?? {}).map((category): CategoryOption => ({
      label: category.name,
      value: category.categoryId,
    })),
  ]);
  const visibleArchives = computed((): ArchiveEntryView[] =>
    archives.value.filter(
      (a): boolean =>
        acceptsMatch(archiveFilter.value, a.scans.length > 0) &&
        (!mapOnly.value || a.acquisitionMethod === 'map') &&
        (archiveCategory.value === ALL_CATEGORIES || a.category === archiveCategory.value) &&
        a.title.includes(archiveSearch.value),
    ),
  );
  const visibleScans = computed((): ScannedItemView[] =>
    scans.value.filter(
      (s): boolean =>
        acceptsMatch(scanFilter.value, s.archives.length > 0) &&
        (scanCategory.value === ALL_CATEGORIES || s.foundInSubCategory === scanCategory.value) &&
        [s.ocrResult, s.correctedTitle ?? ''].some((title): boolean =>
          title.includes(scanSearch.value),
        ),
    ),
  );
  // 新一轮扫描或清空后，旧纠错不再可撤销。
  watch(
    source.scans,
    (items: readonly Readonly<ScannedItemRecord>[]): void => {
      if (
        lastEdit.value &&
        !items.some((s): boolean => s.scannedItemId === lastEdit.value!.scannedItemId)
      )
        lastEdit.value = null;
    },
    { flush: 'sync' },
  );
  /** 返回纠错后的关联档案数量，供页面提示使用。 */
  function correct(id: ScannedItemId, title: string): number {
    const previous = source.scans.value.find((s): boolean => s.scannedItemId === id);
    if (!previous) return 0;
    lastEdit.value = previous;
    source.correct(id, title);
    return view.value.scans.find((s): boolean => s.scannedItemId === id)?.archives.length ?? 0;
  }
  function undo(): void {
    if (!lastEdit.value) return;
    source.restore(lastEdit.value);
    lastEdit.value = null;
  }
  function clear(): void {
    source.clear();
    lastEdit.value = null;
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
    lastEdit,
    correct,
    undo,
    clear,
    revealScan,
    revealArchive,
  };
}
