<script setup lang="ts">
import { useTranslatedToast } from '@/shared/i18n/toast';
import { useAppI18n } from '@/shared/i18n';
import { computed, nextTick, onUnmounted, ref, watch, type Ref } from 'vue';
import { useResizeObserver } from '@vueuse/core';
import type { SplitterItem } from '@nuxt/ui';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  useArchiveScanResults,
  type ArchiveScanSource,
  type ScanEdit,
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
  edits,
  choose: chooseArchive,
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

const workspace: Ref<HTMLElement | null> = ref(null);
const workspaceWidth = ref(0);
const workspaceHeight = ref(0);
const sideBySide = computed((): boolean => workspaceWidth.value >= 648);
useResizeObserver(workspace, (entries): void => {
  workspaceWidth.value = entries[0]!.contentRect.width;
  workspaceHeight.value = entries[0]!.contentRect.height;
});
// 横向保留 300px / 340px 最小宽度，纵向每栏至少 300px，避免挤压工具区和内容。
const splitItems = computed<SplitterItem[]>(() => {
  const availableSize = sideBySide.value
    ? workspaceWidth.value - 8
    : Math.max(workspaceHeight.value - 8, 600);
  return [
    {
      slot: 'archives',
      defaultSize: 55.5,
      minSize: (300 / availableSize) * 100,
    },
    {
      slot: 'scans',
      defaultSize: 44.5,
      minSize: ((sideBySide.value ? 340 : 300) / availableSize) * 100,
    },
  ];
});
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
      if (!state.editing) state.draftArchiveId = item.archive?.id ?? null;
      states[item.scannedItemId] = state;
    }
    cardStates.value = states;
  },
  { immediate: true, flush: 'sync' },
);
watch(edits, (edits: readonly ScanEdit[]): void => {
  const valid = new Set(edits.map((edit: ScanEdit): number => edit.key));
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
function choose(id: ScannedItemId, archiveId: ArchiveId | null): void {
  const editRecord = chooseArchive(id, archiveId);
  const selected = scans.value.find(
    (item: ScannedItemView): boolean => item.scannedItemId === id,
  )!.archive;
  const notification = translatedToast.add(
    {
      duration: 20000,
      color: 'neutral',
      close: { color: 'neutral', variant: 'outline' },
    },
    () => ({
      title: selected
        ? t('scan.assignment', { id, title: selected.title })
        : t('scan.assignmentCleared', { id }),

      actions: [
        {
          label: t('scan.undo'),
          color: 'neutral',
          variant: 'outline',
          onClick: (): void => {
            undo(editRecord);
            const state = cardStates.value[id];
            if (state) state.expanded = undefined;
            toast.remove(notification.id);
          },
        },
      ],
    }),
  );
  editToasts.set(editRecord.key, notification.id);
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
  <!-- 两栏至少需要 300 + 8 + 340px。窄布局沿用同一组列表，保留编辑状态和滚动位置。 -->
  <div
    ref="workspace"
    class="matching-workspace @container/scan flex min-h-0 w-full min-w-0 flex-1 flex-col"
  >
    <USplitter
      id="archive-matching"
      :aria-label="t('scan.resize')"
      class="min-h-[608px] flex-1 @min-[648px]/scan:min-h-0"
      :items="splitItems"
      :keyboard-resize-by="2"
      :orientation="sideBySide ? 'horizontal' : 'vertical'"
      :ui="{
        panel: 'min-h-0 min-w-0 flex-col',
        handle: 'flex items-center justify-center focus-visible:outline-none',
      }"
    >
      <template #archives>
        <section
          :aria-label="t('scan.allArchives')"
          class="flex min-h-0 min-w-0 flex-1 flex-col rounded-xl border border-default bg-elevated/20"
        >
          <header class="space-y-2 border-b border-default p-3">
            <div class="flex items-center justify-between gap-2">
              <h2 class="text-lg font-semibold whitespace-nowrap">
                {{ t('scan.allArchives') }}
                <span class="font-normal text-muted">{{ n(archives.length) }}</span>
              </h2>
              <ArchiveExportButton
                :collected="matchedArchives"
                :disabled="!!source"
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
                  {
                    value: 'matched' as const,
                    label: t('scan.hasRecords'),
                    count: matchedArchives,
                  },
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
                size="xs"
              /><UPopover
                ><UButton
                  :aria-label="t('scan.filters')"
                  :color="mapOnly || archiveCategory !== 'all' ? 'primary' : 'neutral'"
                  icon="i-lucide-list-filter"
                  size="xs"
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
            class="min-h-0 flex-1"
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
      </template>
      <template #resize-handle>
        <span
          class="flex items-center justify-center text-dimmed"
          :class="sideBySide ? 'h-8 w-2' : 'h-2 w-8'"
          :title="t('scan.resize')"
          ><UIcon
            class="size-4 shrink-0"
            :name="sideBySide ? 'i-lucide-ellipsis-vertical' : 'i-lucide-ellipsis'"
        /></span>
      </template>
      <template #scans>
        <section
          :aria-label="t('scan.results')"
          class="flex min-h-0 min-w-0 flex-1 flex-col rounded-xl border border-default bg-elevated/20"
        >
          <header class="space-y-2 border-b border-default p-3">
            <div class="flex items-center justify-between">
              <h2 class="text-lg font-semibold">
                {{ t('scan.results') }}
                <span class="font-normal text-muted">{{ n(scans.length) }}</span>
              </h2>
              <ArchiveScanButton :disabled="!!source" size="sm" />
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
                size="xs"
              /><USelect
                v-model="scanCategory"
                :aria-label="t('scan.scanCategory')"
                class="w-28"
                :items="categories"
                size="xs"
              />
            </div>
          </header>
          <ArchiveVirtualList
            v-if="visibleScans.length"
            ref="scanList"
            v-slot="{ item }"
            class="min-h-0 flex-1 p-2"
            :items="visibleScans"
          >
            <ScannedItemCard
              v-model:state="cardStates[item.scannedItemId]!"
              :item="item"
              :selected="selectedScanId === item.scannedItemId"
              @choose="choose(item.scannedItemId, $event)"
              @clear-highlight="clearArchiveHighlight"
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
      </template>
    </USplitter>
  </div>
</template>
