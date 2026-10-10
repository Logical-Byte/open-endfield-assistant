<script setup lang="ts">
import { useTranslatedToast } from '@/shared/i18n/toast';
import { useAppI18n } from '@/shared/i18n';
import { computed, nextTick, onUnmounted, ref, watch, type Ref } from 'vue';
import { useResizeObserver } from '@vueuse/core';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  useArchiveScanResults,
  type ArchiveScanSource,
  type ScanCorrection,
} from '@/features/archiveScan/useArchiveScanResults';
import type { ArchiveId } from '@/shared/types/archive';
import type { ScannedItemId } from '@/features/archiveScan/types/scannedItem';
import ArchiveEntryCard from './ArchiveEntryCard.vue';
import ScannedItemCard from './ScannedItemCard.vue';
import type { ArchiveEntryView, ScannedItemView } from '@/features/archiveScan/resultView';
import ArchiveVirtualList from './ArchiveVirtualList.vue';
import type { Virtualizer } from '@tanstack/vue-virtual';
type ScrollList = { virtualizer: Virtualizer<HTMLElement, Element> };
import { createScanCardState, type ScanCardState } from './scanCardState';

const { t, n } = useAppI18n();

const props = defineProps<{ source?: ArchiveScanSource }>();
const {
  archives,
  scans,
  visibleArchives,
  visibleScans,
  matchedArchives,
  matchedScans,
  categories,
  archiveFilter,
  scanFilter,
  archiveSearch,
  scanSearch,
  archiveCategory,
  scanCategory,
  mapOnly,
  corrections,
  correct: correctTitle,
  undo,
  clear: clearRecords,
  revealScan,
  revealArchive,
} = useArchiveScanResults(props.source);
const { isActive } = useAutomationTask('archiveScan');
const toast = useToast();
const translatedToast = useTranslatedToast();
const selectedArchiveId: Ref<ArchiveId | null> = ref(null);
const selectedScanId: Ref<ScannedItemId | null> = ref(null);
const archiveList: Ref<ScrollList | null> = ref(null);
const scanList: Ref<ScrollList | null> = ref(null);

const splitContainer: Ref<HTMLElement | null> = ref(null);
const leftWidth: Ref<number> = ref(55.5);
const resizing: Ref<boolean> = ref(false);
const splitStyle = computed((): Record<string, string> => ({
  '--left-pane': `${leftWidth.value}fr`,
  '--right-pane': `${100 - leftWidth.value}fr`,
}));
// 下限与 CSS 的两栏最小宽度一致，拖动时保留标题和操作区所需空间。
function resizeLimits(): { min: number; max: number } {
  const width = (splitContainer.value?.clientWidth ?? 0) - 16;
  const archiveMin = 300;
  return width >= archiveMin + 340
    ? { min: (archiveMin / width) * 100, max: ((width - 340) / width) * 100 }
    : { min: 28, max: 72 };
}
function clampSplit(value: number): number {
  const { min, max } = resizeLimits();
  return Math.min(max, Math.max(min, value));
}
useResizeObserver(splitContainer, (): void => {
  leftWidth.value = clampSplit(leftWidth.value);
});
function resizeStart(event: PointerEvent): void {
  if (event.button !== 0) return;
  event.preventDefault();
  resizing.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}
function resizeMove(event: PointerEvent): void {
  if (!resizing.value) return;
  const rect = splitContainer.value!.getBoundingClientRect();
  leftWidth.value = clampSplit(((event.clientX - rect.left - 8) / (rect.width - 16)) * 100);
}
function resizeEnd(): void {
  resizing.value = false;
}
function resizeKeyboard(event: KeyboardEvent): void {
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    event.preventDefault();
    leftWidth.value = clampSplit(leftWidth.value + (event.key === 'ArrowLeft' ? -2 : 2));
  }
}
function clearScanHighlight(): void {
  selectedScanId.value = null;
}
function clearArchiveHighlight(): void {
  selectedArchiveId.value = null;
}
// 每张卡片的草稿由列表持有，虚拟滚动卸载组件不会放弃编辑。
const cardStates = ref<Record<number, ScanCardState>>({});
const editToasts = new Map<number, string | number>();
watch(
  scans,
  (items: readonly ScannedItemView[]): void => {
    const states: Record<number, ScanCardState> = {};
    for (const item of items) {
      const state = cardStates.value[item.scannedItemId] ?? createScanCardState(item);
      if (!state.editing) state.draft = item.correctedTitle ?? item.ocrResult;
      states[item.scannedItemId] = state;
    }
    cardStates.value = states;
  },
  { immediate: true, flush: 'sync' },
);
watch(corrections, (edits: readonly ScanCorrection[]): void => {
  const valid = new Set(edits.map((edit: ScanCorrection): number => edit.key));
  for (const [key, id] of editToasts) {
    if (!valid.has(key)) {
      toast.remove(id);
      editToasts.delete(key);
    }
  }
});
onUnmounted((): void => {
  for (const id of editToasts.values()) toast.remove(id);
});
function correct(id: ScannedItemId, title: string): void {
  const correction = correctTitle(id, title);
  const matched =
    scans.value.find((item: ScannedItemView): boolean => item.scannedItemId === id)!.archives
      .length > 0;
  const notification = translatedToast.add(
    {
      duration: 20000,
      color: 'neutral',
      close: { color: 'neutral', variant: 'outline' },
    },
    () => ({
      title: t('scan.correction', { id, title }),
      description: matched ? undefined : t('scan.titleUnmatched'),
      actions: [
        {
          label: t('scan.undo'),
          color: 'neutral',
          variant: 'outline',
          onClick: (): void => {
            undo(correction);
            const state = cardStates.value[id];
            if (state) state.expanded = undefined;
            toast.remove(notification.id);
          },
        },
      ],
    }),
  );
  editToasts.set(correction.key, notification.id);
}
// 解除筛选后先等虚拟列表收到新 items，再按索引定位未挂载的记录。
async function locateScan(id: ScannedItemId): Promise<void> {
  revealScan();
  selectedScanId.value = id;
  cardStates.value[id]!.expanded = true;
  await nextTick();
  const index = visibleScans.value.findIndex(
    (scan: ScannedItemView): boolean => scan.scannedItemId === id,
  );
  if (index >= 0) scanList.value?.virtualizer?.scrollToIndex(index, { align: 'start' });
}
async function locateArchive(id: ArchiveId): Promise<void> {
  revealArchive();
  selectedArchiveId.value = id;
  await nextTick();
  const index = visibleArchives.value.findIndex(
    (archive: ArchiveEntryView): boolean => archive.id === id,
  );
  if (index >= 0) archiveList.value?.virtualizer?.scrollToIndex(index, { align: 'start' });
}
function clearScans(): void {
  clearRecords();
  selectedArchiveId.value = null;
  selectedScanId.value = null;
}
watch(scans, (): void => {
  if (
    !scans.value.some(
      (scan: ScannedItemView): boolean => scan.scannedItemId === selectedScanId.value,
    )
  )
    selectedScanId.value = null;
  if (!scans.value.length) selectedArchiveId.value = null;
});
</script>

<template>
  <div class="matching-workspace flex min-h-0 w-full flex-1 flex-col">
    <div
      ref="splitContainer"
      class="grid min-h-0 gap-3 min-[848px]:flex-1 min-[848px]:grid-cols-[minmax(300px,var(--left-pane))_16px_minmax(340px,var(--right-pane))] min-[848px]:gap-0"
      :class="resizing && 'select-none'"
      :style="splitStyle"
    >
      <section
        :aria-label="t('scan.allArchives')"
        class="min-w-0 rounded-xl border border-default bg-elevated/20 min-[848px]:flex min-[848px]:min-h-0 min-[848px]:flex-col"
      >
        <header class="space-y-2 border-b border-default p-3">
          <div class="flex items-center justify-between gap-2">
            <h2 class="text-lg font-semibold whitespace-nowrap">
              {{ t('scan.allArchives') }}
              <span class="font-normal text-muted">{{ n(archives.length) }}</span>
            </h2>
            <ArchiveExportButton
              :collected="matchedArchives"
              :total="archives.length"
              :unmatched="scans.length - matchedScans"
            />
          </div>
          <div class="flex items-center gap-1">
            <UButton
              v-for="option in [
                { value: 'all' as const, label: t('scan.all'), count: archives.length },
                {
                  value: 'unmatched' as const,
                  label: t('scan.noRecords'),
                  count: archives.length - matchedArchives,
                },
                { value: 'matched' as const, label: t('scan.hasRecords'), count: matchedArchives },
              ]"
              :key="option.value"
              :color="archiveFilter === option.value ? 'primary' : 'neutral'"
              size="xs"
              :variant="archiveFilter === option.value ? 'subtle' : 'outline'"
              @click="archiveFilter = option.value"
              >{{ option.label }} {{ n(option.count) }}</UButton
            >
          </div>
          <div class="flex gap-2">
            <UInput
              v-model="archiveSearch"
              :aria-label="t('scan.searchArchive')"
              class="min-w-0 flex-1"
              icon="i-lucide-search"
              :placeholder="t('scan.searchArchive')"
              size="sm"
            /><UPopover
              ><UButton
                :aria-label="t('scan.filters')"
                :color="mapOnly || archiveCategory !== 'all' ? 'primary' : 'neutral'"
                icon="i-lucide-list-filter"
                size="sm"
                variant="outline" /><template #content
                ><div class="w-64 space-y-3 p-3">
                  <USelect
                    v-model="archiveCategory"
                    :aria-label="t('scan.archiveCategory')"
                    class="w-full"
                    :items="categories"
                  /><UCheckbox v-model="mapOnly" :label="t('scan.mapOnly')" /></div></template
            ></UPopover>
          </div>
        </header>
        <ArchiveVirtualList
          v-if="visibleArchives.length"
          ref="archiveList"
          v-slot="{ item: archive }"
          archive-rows
          class="max-h-[65vh] min-h-0 flex-1 min-[848px]:max-h-none"
          :items="visibleArchives"
        >
          <ArchiveEntryCard
            :entry="archive"
            :selected="selectedArchiveId === archive.id"
            @clear-highlight="clearScanHighlight"
            @locate-scan="locateScan"
          />
        </ArchiveVirtualList>
        <p v-if="!visibleArchives.length" class="flex-1 py-14 text-center text-sm text-muted">
          {{ t('scan.noArchives') }}
        </p>
        <div class="shrink-0 border-t border-default px-3 py-1.5 text-[11px] text-muted">
          {{
            t('scan.archiveCount', {
              visible: n(visibleArchives.length),
              total: n(archives.length),
            })
          }}
        </div>
      </section>

      <div
        :aria-label="t('scan.resize')"
        aria-orientation="vertical"
        :aria-valuemax="Math.round(resizeLimits().max)"
        :aria-valuemin="Math.round(resizeLimits().min)"
        :aria-valuenow="Math.round(leftWidth)"
        class="split-divider group relative hidden cursor-col-resize touch-none items-center justify-center outline-none min-[848px]:flex"
        :class="resizing && 'bg-primary/10'"
        role="separator"
        tabindex="0"
        @keydown="resizeKeyboard"
        @lostpointercapture="resizeEnd"
        @pointercancel="resizeEnd"
        @pointerdown="resizeStart"
        @pointermove="resizeMove"
        @pointerup="resizeEnd"
      >
        <span
          class="h-full w-px bg-default transition-colors group-hover:bg-primary group-focus-visible:bg-primary"
        /><span
          class="absolute flex h-10 w-3 items-center justify-center rounded-full border border-default bg-elevated text-muted group-hover:border-primary group-hover:text-primary"
          ><UIcon class="size-3" name="i-lucide-grip-vertical"
        /></span>
      </div>
      <section
        :aria-label="t('scan.results')"
        class="min-w-0 rounded-xl border border-default bg-elevated/20 min-[848px]:flex min-[848px]:min-h-0 min-[848px]:flex-col"
      >
        <header class="space-y-2 border-b border-default p-3">
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-semibold">
              {{ t('scan.results') }}
              <span class="font-normal text-muted">{{ n(scans.length) }}</span>
            </h2>
            <ArchiveScanButton size="sm" />
          </div>
          <div class="flex items-center gap-1">
            <UButton
              v-for="option in [
                {
                  value: 'unmatched' as const,
                  label: t('scan.review'),
                  count: scans.length - matchedScans,
                },
                { value: 'matched' as const, label: t('scan.matched'), count: matchedScans },
                { value: 'all' as const, label: t('scan.all'), count: scans.length },
              ]"
              :key="option.value"
              :color="scanFilter === option.value ? 'primary' : 'neutral'"
              size="xs"
              :variant="scanFilter === option.value ? 'subtle' : 'outline'"
              @click="scanFilter = option.value"
              >{{ option.label }} {{ n(option.count) }}</UButton
            ><UTooltip :text="t('scan.clearResults')"
              ><UButton
                :aria-label="t('scan.clearResults')"
                class="ml-auto"
                color="neutral"
                :disabled="isActive"
                icon="i-lucide-trash-2"
                :label="t('scan.clear')"
                size="xs"
                variant="outline"
                @click="clearScans"
            /></UTooltip>
          </div>
          <div class="flex gap-2">
            <UInput
              v-model="scanSearch"
              :aria-label="t('scan.searchResults')"
              class="min-w-0 flex-1"
              icon="i-lucide-search"
              :placeholder="t('scan.searchTitle')"
              size="sm"
            /><USelect
              v-model="scanCategory"
              :aria-label="t('scan.scanCategory')"
              class="w-28"
              :items="categories"
              size="sm"
            />
          </div>
        </header>
        <ArchiveVirtualList
          v-if="visibleScans.length"
          ref="scanList"
          v-slot="{ item }"
          class="max-h-[65vh] min-h-0 flex-1 p-2 min-[848px]:max-h-none"
          :items="visibleScans"
        >
          <ScannedItemCard
            v-model:state="cardStates[item.scannedItemId]!"
            :item="item"
            :selected="selectedScanId === item.scannedItemId"
            @clear-highlight="clearArchiveHighlight"
            @correct="correct(item.scannedItemId, $event)"
            @locate-archive="locateArchive"
          />
        </ArchiveVirtualList>
        <div v-if="!visibleScans.length" class="flex-1 py-14 text-center">
          <p class="text-sm text-muted">
            {{
              !scans.length
                ? t('scan.startHint')
                : scanFilter === 'unmatched' && matchedScans === scans.length
                  ? t('scan.allMatched')
                  : t('scan.noScans')
            }}
          </p>
        </div>
        <div class="shrink-0 border-t border-default px-3 py-1.5 text-[11px] text-muted">
          {{ t('scan.scanCount', { visible: n(visibleScans.length), total: n(scans.length) }) }}
        </div>
      </section>
    </div>
  </div>
</template>
