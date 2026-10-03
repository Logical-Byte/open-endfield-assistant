<script setup lang="ts">
import { useEvidencePopover } from './useEvidencePopover';
import type { ArchiveEntryView } from '@/features/archiveScan/resultView';
import type { ScannedItemId } from '@/features/archiveScan/types/scannedItem';
defineProps<{
  entry: ArchiveEntryView;
  selected?: boolean;
}>();
const emit = defineEmits<{ clearHighlight: []; locateScan: [id: ScannedItemId] }>();
const {
  open: evidenceOpen,
  cancelClose,
  scheduleClose,
} = useEvidencePopover((): void => emit('clearHighlight'));
</script>

<template>
  <article
    class="archive-card relative isolate overflow-hidden rounded-lg border transition-colors"
    :class="[
      selected ? 'border-primary bg-primary/5' : 'border-default bg-default',
      'gradient-surface',
    ]"
    :style="{
      '--matching-tint': entry.scans.length
        ? 'var(--matching-matched-background)'
        : 'var(--matching-unknown-background)',
    }"
  >
    <div class="archive-layout px-3 py-2.5">
      <div class="archive-info min-w-0">
        <UBadge
          class="self-start"
          :color="entry.scans.length ? 'success' : 'neutral'"
          size="sm"
          variant="soft"
          >{{ entry.scans.length ? '有匹配' : '未匹配' }}</UBadge
        >
        <div class="min-w-0">
          <h3 class="line-clamp-2 text-sm font-medium text-highlighted" :title="entry.title">
            {{ entry.title }}
          </h3>
          <p class="mt-1 truncate text-xs text-muted" :title="entry.categoryLabel">
            {{ entry.categoryLabel }}
          </p>
        </div>
      </div>
      <div class="archive-actions">
        <UPopover
          v-if="entry.scans.length"
          v-model:open="evidenceOpen"
          :content="{ side: 'bottom', align: 'end', sideOffset: 5 }"
          mode="click"
          :ui="{ content: 'w-80 max-w-[calc(100vw-24px)] p-3' }"
        >
          <UButton
            :aria-expanded="evidenceOpen"
            class="h-7 w-32 justify-center"
            color="neutral"
            icon="i-lucide-link"
            :label="`${entry.scans.length} 份扫描证据`"
            size="xs"
            trailing-icon="i-lucide-chevron-down"
            variant="ghost"
            @mouseenter="cancelClose"
            @mouseleave="scheduleClose"
          />
          <template #content>
            <div @mouseenter="cancelClose" @mouseleave="scheduleClose">
              <p class="mb-2 text-sm font-medium">{{ entry.title }}</p>
              <p class="mb-2 text-[11px] text-muted">{{ entry.scans.length }} 份扫描证据</p>
              <div class="max-h-64 space-y-1.5 overflow-y-auto">
                <div
                  v-for="scan in entry.scans"
                  :key="scan.scannedItemId"
                  class="flex items-center gap-2 rounded border border-default px-2 py-1.5"
                >
                  <div class="min-w-0 flex-1">
                    <p class="text-xs text-toned">
                      #{{ scan.scannedItemId }} ·
                      {{ scan.manuallyCorrected ? '人工纠正' : '自动匹配' }}
                    </p>
                    <p class="mt-0.5 truncate text-xs text-muted">
                      OCR：{{ scan.ocrResult || '无文字' }}
                    </p>
                  </div>
                  <UButton
                    color="neutral"
                    label="定位记录"
                    size="xs"
                    trailing-icon="i-lucide-arrow-right"
                    variant="ghost"
                    @click="emit('locateScan', scan.scannedItemId)"
                  />
                </div>
              </div>
            </div>
          </template>
        </UPopover>
        <span v-else class="flex h-7 w-32 items-center justify-center text-xs text-dimmed"
          >暂无扫描证据</span
        >
        <UButton
          v-if="entry.acquisitionMethod !== null"
          class="h-7 w-32 justify-center"
          color="neutral"
          :label="
            entry.acquisitionMethod === 'map'
              ? entry.scans.length
                ? '在 OEM 中查看'
                : '前往 OEM 收集'
              : (entry.acquisitionLabel ?? '获取方式未知')
          "
          size="xs"
          target="_blank"
          :to="entry.acquisitionMethod === 'map' ? entry.oemUrl : entry.intelUrl"
          trailing-icon="i-lucide-external-link"
          variant="outline"
        />
        <span v-else class="flex h-7 w-32 items-center justify-center text-xs text-dimmed"
          >获取方式未知</span
        >
      </div>
    </div>
  </article>
</template>

<style scoped>
.archive-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 128px;
  gap: 8px;
  align-items: center;
}
.archive-info {
  min-height: 60px;
  align-content: center;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 6px;
  align-items: start;
}
.archive-actions {
  display: grid;
  grid-template-columns: 128px;
  gap: 2px;
  align-items: center;
  justify-content: center;
}
.gradient-surface {
  background-image: linear-gradient(
    90deg,
    color-mix(in srgb, var(--matching-tint) 7%, transparent) 12%,
    transparent 45%
  );
}
@media (min-width: 1200px) {
  .archive-info h3 {
    display: block;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .archive-info {
    min-height: 0;
  }
  .archive-layout {
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .archive-actions {
    grid-template-columns: 128px 128px;
    gap: 4px;
  }
}
</style>
