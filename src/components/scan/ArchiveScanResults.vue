<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch, type Ref } from 'vue';
import { useResizeObserver } from '@vueuse/core';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  useArchiveScanResults,
  type ArchiveScanSource,
  type ScanCorrection,
} from '@/features/archiveScan/useArchiveScanResults';
import type { ArchiveId, ScannedItemId } from '@/features/archiveScan/types/scannedItem';
import ArchiveEntryCard from './ArchiveEntryCard.vue';
import ScannedItemCard from './ScannedItemCard.vue';
import type { ArchiveEntryView, ScannedItemView } from '@/features/archiveScan/resultView';
import ArchiveVirtualList from './ArchiveVirtualList.vue';
import type { Virtualizer } from '@tanstack/vue-virtual';
type ScrollList = { virtualizer: Virtualizer<HTMLElement, Element> };
import { createScanCardState, type ScanCardState } from './scanCardState';

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
  const notification = toast.add({
    title: `将第 ${id} 个识别结果编辑为“${title}”。`,
    description: matched ? undefined : '标题仍未匹配到已知档案。',
    duration: 20000,
    color: 'neutral',
    close: { color: 'neutral', variant: 'outline' },
    actions: [
      {
        label: '撤销编辑',
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
  });
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
        aria-label="全部档案"
        class="min-w-0 rounded-xl border border-default bg-elevated/20 min-[848px]:flex min-[848px]:min-h-0 min-[848px]:flex-col"
      >
        <header class="space-y-2 border-b border-default p-3">
          <div class="flex items-center justify-between gap-2">
            <h2 class="text-lg font-semibold whitespace-nowrap">
              全部档案 <span class="font-normal text-muted">{{ archives.length }}</span>
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
                { value: 'all' as const, label: '全部', count: archives.length },
                {
                  value: 'unmatched' as const,
                  label: '无记录',
                  count: archives.length - matchedArchives,
                },
                { value: 'matched' as const, label: '有记录', count: matchedArchives },
              ]"
              :key="option.value"
              :color="archiveFilter === option.value ? 'primary' : 'neutral'"
              size="xs"
              :variant="archiveFilter === option.value ? 'subtle' : 'outline'"
              @click="archiveFilter = option.value"
              >{{ option.label }} {{ option.count }}</UButton
            >
          </div>
          <div class="flex gap-2">
            <UInput
              v-model="archiveSearch"
              aria-label="搜索档案标题"
              class="min-w-0 flex-1"
              icon="i-lucide-search"
              placeholder="搜索档案标题"
              size="sm"
            /><UPopover
              ><UButton
                aria-label="目录筛选"
                :color="mapOnly || archiveCategory !== 'all' ? 'primary' : 'neutral'"
                icon="i-lucide-list-filter"
                size="sm"
                variant="outline" /><template #content
                ><div class="w-64 space-y-3 p-3">
                  <USelect
                    v-model="archiveCategory"
                    aria-label="档案分类"
                    class="w-full"
                    :items="categories"
                  /><UCheckbox v-model="mapOnly" label="仅显示可在地图拾取的档案" /></div></template
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
          当前筛选下没有档案
        </p>
        <div class="shrink-0 border-t border-default px-3 py-1.5 text-[11px] text-muted">
          显示 {{ visibleArchives.length }} / {{ archives.length }} 份档案
        </div>
      </section>

      <div
        aria-label="调整全部档案与扫描结果宽度"
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
        aria-label="扫描结果"
        class="min-w-0 rounded-xl border border-default bg-elevated/20 min-[848px]:flex min-[848px]:min-h-0 min-[848px]:flex-col"
      >
        <header class="space-y-2 border-b border-default p-3">
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-semibold">
              扫描结果 <span class="font-normal text-muted">{{ scans.length }}</span>
            </h2>
            <ArchiveScanButton size="sm" />
          </div>
          <div class="flex items-center gap-1">
            <UButton
              v-for="option in [
                {
                  value: 'unmatched' as const,
                  label: '待核对',
                  count: scans.length - matchedScans,
                },
                { value: 'matched' as const, label: '已匹配', count: matchedScans },
                { value: 'all' as const, label: '全部', count: scans.length },
              ]"
              :key="option.value"
              :color="scanFilter === option.value ? 'primary' : 'neutral'"
              size="xs"
              :variant="scanFilter === option.value ? 'subtle' : 'outline'"
              @click="scanFilter = option.value"
              >{{ option.label }} {{ option.count }}</UButton
            ><UTooltip text="清空扫描结果"
              ><UButton
                aria-label="清空扫描结果"
                class="ml-auto"
                color="neutral"
                :disabled="isActive"
                icon="i-lucide-trash-2"
                label="清空"
                size="xs"
                variant="outline"
                @click="clearScans"
            /></UTooltip>
          </div>
          <div class="flex gap-2">
            <UInput
              v-model="scanSearch"
              aria-label="搜索扫描结果"
              class="min-w-0 flex-1"
              icon="i-lucide-search"
              placeholder="搜索识别或修正标题"
              size="sm"
            /><USelect
              v-model="scanCategory"
              aria-label="扫描分类"
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
                ? '开始扫描后，结果会出现在这里'
                : scanFilter === 'unmatched' && matchedScans === scans.length
                  ? '所有扫描结果都已匹配'
                  : '当前筛选下没有扫描结果'
            }}
          </p>
        </div>
        <div class="shrink-0 border-t border-default px-3 py-1.5 text-[11px] text-muted">
          显示 {{ visibleScans.length }} / {{ scans.length }} 条记录
        </div>
      </section>
    </div>
  </div>
</template>
