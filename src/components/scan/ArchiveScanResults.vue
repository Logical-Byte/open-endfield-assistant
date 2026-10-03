<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useResizeObserver } from '@vueuse/core';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  useArchiveScanResults,
  filterOptions,
  type ArchiveScanSource,
} from '@/features/archiveScan/useArchiveScanResults';
import type { ArchiveId, ScannedItemId } from '@/features/archiveScan/types/scannedItem';
import ArchiveEntryCard from './ArchiveEntryCard.vue';
import ScannedItemCard from './ScannedItemCard.vue';

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
  lastEdit,
  correct: correctTitle,
  undo,
  clear: clearRecords,
  revealScan,
  revealArchive,
} = useArchiveScanResults(props.source);
const { isActive } = useAutomationTask('archiveScan');
const toast = useToast();
const selectedArchiveId = ref<ArchiveId | null>(null);
const selectedScanId = ref<ScannedItemId | null>(null);
// Nuxt UI 暴露的 virtualizer 负责将尚未挂载的记录滚入可见区。
interface ScrollList {
  virtualizer?: { scrollToIndex: (index: number, options: { align: 'auto' }) => void };
}
const archiveList = ref<ScrollList | null>(null);
const scanList = ref<ScrollList | null>(null);
const splitContainer = ref<HTMLElement | null>(null);
const leftWidth = ref(46.5);
const resizing = ref(false);
const splitStyle = computed((): Record<string, string> => ({
  '--left-pane': `${leftWidth.value}fr`,
  '--right-pane': `${100 - leftWidth.value}fr`,
}));
// 下限与 CSS 的两栏最小宽度一致，拖动时保留标题和操作区所需空间。
function resizeLimits(): { min: number; max: number } {
  const width = (splitContainer.value?.clientWidth ?? 0) - 16;
  const archiveMin = window.innerWidth >= 1200 ? 560 : 360;
  return width >= archiveMin + 420
    ? { min: (archiveMin / width) * 100, max: ((width - 420) / width) * 100 }
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
function correct(id: ScannedItemId, title: string): void {
  const count = correctTitle(id, title);
  toast.add({
    title: count ? `已关联 ${count} 份档案` : '标题仍未匹配',
    description: title,
    color: count ? 'success' : 'warning',
  });
}
// 解除筛选后先等虚拟列表收到新 items，再按索引定位未挂载的记录。
async function locateScan(id: ScannedItemId): Promise<void> {
  revealScan();
  selectedScanId.value = id;
  await nextTick();
  const index = visibleScans.value.findIndex((scan): boolean => scan.scannedItemId === id);
  if (index >= 0) scanList.value?.virtualizer?.scrollToIndex(index, { align: 'auto' });
}
async function locateArchive(id: ArchiveId): Promise<void> {
  revealArchive();
  selectedArchiveId.value = id;
  await nextTick();
  const index = visibleArchives.value.findIndex((archive): boolean => archive.id === id);
  if (index >= 0) archiveList.value?.virtualizer?.scrollToIndex(index, { align: 'auto' });
}
function clearScans(): void {
  clearRecords();
  selectedArchiveId.value = null;
  selectedScanId.value = null;
}
watch(scans, (): void => {
  if (!scans.value.some((scan): boolean => scan.scannedItemId === selectedScanId.value))
    selectedScanId.value = null;
  if (!scans.value.length) selectedArchiveId.value = null;
});
</script>

<template>
  <div class="matching-workspace flex min-h-0 w-full flex-1 flex-col">
    <div
      ref="splitContainer"
      class="resizable-columns grid min-h-0"
      :class="resizing && 'select-none'"
      :style="splitStyle"
    >
      <section
        aria-label="档案目录"
        class="matching-pane min-w-0 rounded-xl border border-default bg-elevated/20"
      >
        <div class="shrink-0 border-b border-default px-3 pt-2 pb-2">
          <div class="mb-1.5 flex min-h-7 items-center justify-between">
            <h2 class="text-sm font-semibold text-highlighted">
              档案目录
              <span class="ml-1 text-xs font-normal text-muted">{{ archives.length }} 份</span>
            </h2>
          </div>
          <div class="mb-1.5 flex flex-wrap gap-1">
            <UButton
              v-for="option in filterOptions(archives.length, matchedArchives)"
              :key="option.value"
              :color="archiveFilter === option.value ? 'primary' : 'neutral'"
              :label="`${option.label} ${option.count}`"
              size="xs"
              :variant="archiveFilter === option.value ? 'soft' : 'ghost'"
              @click="archiveFilter = option.value"
            />
          </div>
          <div class="flex gap-2">
            <UInput
              v-model="archiveSearch"
              aria-label="搜索档案标题"
              class="min-w-0 flex-1"
              icon="i-lucide-search"
              placeholder="搜索档案标题"
              size="sm"
            /><USelect
              v-model="archiveCategory"
              aria-label="档案目录分类"
              class="w-28"
              :items="categories"
              size="sm"
            />
          </div>
          <UCheckbox
            v-model="mapOnly"
            class="mt-1.5"
            color="info"
            label="隐藏无法在大世界中获取的档案"
            size="sm"
          />
        </div>
        <UScrollArea
          v-if="visibleArchives.length"
          ref="archiveList"
          v-slot="{ item: archive }"
          class="column-list min-h-0 flex-1 p-2"
          :items="visibleArchives"
          :virtualize="{
            estimateSize: 88,
            paddingStart: 8,
            paddingEnd: 8,
            gap: 6,
            overscan: 8,
            getItemKey: (index: number): string => visibleArchives[index]!.id,
          }"
        >
          <ArchiveEntryCard
            :entry="archive"
            :selected="selectedArchiveId === archive.id"
            @clear-highlight="clearScanHighlight"
            @locate-scan="locateScan"
          />
        </UScrollArea>
        <p v-if="!visibleArchives.length" class="flex-1 py-14 text-center text-sm text-muted">
          当前筛选下没有档案
        </p>
        <div class="shrink-0 border-t border-default px-3 py-1.5 text-[11px] text-muted">
          显示 {{ visibleArchives.length }} / {{ archives.length }} 份档案
        </div>
      </section>

      <div
        aria-label="调整档案目录与扫描记录宽度"
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
        aria-label="扫描记录"
        class="matching-pane min-w-0 rounded-xl border border-default bg-elevated/20"
      >
        <div class="shrink-0 border-b border-default px-3 pt-2 pb-2">
          <div class="mb-1.5 flex min-h-7 items-center justify-between">
            <h2 class="text-sm font-semibold text-highlighted">
              扫描记录
              <span class="ml-1 text-xs font-normal text-muted">{{ scans.length }} 条</span>
            </h2>
            <div class="flex items-center gap-1">
              <UButton
                v-if="lastEdit"
                color="neutral"
                icon="i-lucide-undo-2"
                label="撤销纠错"
                size="xs"
                variant="ghost"
                @click="undo"
              />
              <UButton
                color="error"
                :disabled="isActive"
                icon="i-lucide-trash-2"
                label="清空"
                size="xs"
                variant="ghost"
                @click="clearScans"
              />
            </div>
          </div>
          <div class="mb-1.5 flex flex-wrap gap-1">
            <UButton
              v-for="option in filterOptions(scans.length, matchedScans)"
              :key="option.value"
              :color="scanFilter === option.value ? 'primary' : 'neutral'"
              :label="`${option.label} ${option.count}`"
              size="xs"
              :variant="scanFilter === option.value ? 'soft' : 'ghost'"
              @click="scanFilter = option.value"
            />
          </div>
          <div class="flex gap-2">
            <UInput
              v-model="scanSearch"
              aria-label="搜索扫描记录"
              class="min-w-0 flex-1"
              icon="i-lucide-search"
              placeholder="搜索识别或修正标题"
              size="sm"
            /><USelect
              v-model="scanCategory"
              aria-label="扫描记录分类"
              class="w-28"
              :items="categories"
              size="sm"
            />
          </div>
        </div>
        <UScrollArea
          v-if="visibleScans.length"
          ref="scanList"
          v-slot="{ item }"
          class="column-list min-h-0 flex-1 p-2"
          :items="visibleScans"
          :virtualize="{
            estimateSize: 124,
            paddingStart: 8,
            paddingEnd: 8,
            gap: 6,
            overscan: 8,
            getItemKey: (index: number): number => visibleScans[index]!.scannedItemId,
          }"
        >
          <ScannedItemCard
            :item="item"
            :selected="selectedScanId === item.scannedItemId"
            @clear-highlight="clearArchiveHighlight"
            @correct="correct(item.scannedItemId, $event)"
            @locate-archive="locateArchive"
          />
        </UScrollArea>
        <div v-if="!visibleScans.length" class="flex-1 py-14 text-center">
          <p class="text-sm text-muted">
            {{
              !scans.length
                ? '暂无扫描记录，开始扫描后将在这里显示'
                : scanFilter === 'unmatched' && matchedScans === scans.length
                  ? '所有扫描记录都有匹配'
                  : '当前筛选下没有扫描记录'
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

<style scoped>
.matching-workspace {
  --matching-matched-background: hsl(142 71% 30%);
  --matching-unknown-background: hsl(215 16% 47%);
}
:global(.dark .matching-workspace) {
  --matching-matched-background: hsl(142 71% 45%);
}
.column-list {
  max-height: 65vh;
}
.resizable-columns {
  gap: 12px;
}
@media (min-width: 848px) {
  .resizable-columns {
    flex: 1;
    grid-template-columns: minmax(360px, var(--left-pane)) 16px minmax(420px, var(--right-pane));
    gap: 0;
  }
  .matching-pane {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .column-list {
    flex: 1;
    min-height: 0;
    max-height: none;
  }
}
@media (min-width: 1200px) {
  .resizable-columns {
    grid-template-columns: minmax(560px, var(--left-pane)) 16px minmax(420px, var(--right-pane));
  }
}
</style>
