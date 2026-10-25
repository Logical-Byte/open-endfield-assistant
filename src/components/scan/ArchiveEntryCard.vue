<script setup lang="ts">
import { onUnmounted, useId, watch } from 'vue';
import { useAppI18n } from '@/shared/i18n';
import type { ArchiveEntryView } from '@/features/archiveScan/resultView';
import type { ScannedItemId } from '@/features/archiveScan/types/scannedItem';
import ArchiveLabel from './ArchiveLabel.vue';

defineProps<{ entry: ArchiveEntryView; selected?: boolean }>();
const expanded = defineModel<boolean>('expanded', { default: false });
const emit = defineEmits<{
  locateScan: [id: ScannedItemId];
  clearHighlight: [id: ScannedItemId];
}>();
const { t } = useAppI18n();
const bodyId = useId();
let locatedScan: ScannedItemId | null = null;

function locateScan(id: ScannedItemId): void {
  locatedScan = id;
  emit('locateScan', id);
}
function clearHighlight(): void {
  if (locatedScan === null) return;
  emit('clearHighlight', locatedScan);
  locatedScan = null;
}
function clearHighlightOnFocusLeave(event: FocusEvent): void {
  if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null)) {
    clearHighlight();
  }
}
watch(expanded, (value: boolean): void => {
  if (!value) clearHighlight();
});
// 虚拟滚动或筛选卸载来源卡片时，也不能留下对侧的定位高亮。
onUnmounted(clearHighlight);
</script>

<template>
  <UCard
    as="article"
    class="archive-row"
    :class="selected && 'ring-2 ring-primary'"
    :ui="{
      header: 'p-0 sm:px-0',
      body: expanded ? 'p-3 sm:p-3' : 'hidden',
    }"
    @focusout="clearHighlightOnFocusLeave"
    @mouseleave="clearHighlight"
  >
    <template #header>
      <UButton
        :aria-controls="bodyId"
        :aria-expanded="expanded"
        :aria-label="
          t(expanded ? 'scan.collapseArchive' : 'scan.expandArchive', { title: entry.title })
        "
        class="min-h-10 w-full min-w-0 rounded-none px-3 py-2 text-left text-sm"
        color="neutral"
        variant="ghost"
        @click="expanded = !expanded"
      >
        <span class="sr-only">{{
          entry.scans.length ? t('scan.collected') : t('scan.notCollected')
        }}</span>
        <ArchiveLabel
          class="min-w-0 flex-1"
          :content-preview="entry.contentPreview"
          :preview="expanded ? 'hidden' : 'same-line'"
          :title="entry.title"
          variant="plain"
        />
        <UIcon
          v-if="entry.scans.length"
          class="size-4 shrink-0 text-success"
          name="i-lucide-circle-check"
        />
        <UIcon
          class="size-4 shrink-0"
          :name="expanded ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
        />
      </UButton>
    </template>
    <dl :id="bodyId" class="space-y-3 text-sm">
      <div v-if="entry.contentPreview" class="space-y-1">
        <dt class="text-muted">{{ t('scan.contentPreview') }}</dt>
        <dd>{{ entry.contentPreview }}</dd>
      </div>
      <div class="space-y-1">
        <dt class="text-muted">{{ t('scan.archiveCategory') }}</dt>
        <dd>{{ entry.categoryLabel }}</dd>
      </div>
      <div class="space-y-1">
        <dt class="text-muted">{{ t('scan.acquisitionMethod') }}</dt>
        <dd class="flex flex-wrap items-center gap-2">
          <span>
            {{ entry.acquisitionMethod ? t(entry.acquisitionLabel) : t('scan.unknownAcquisition') }}
          </span>
          <UButton
            v-if="entry.acquisitionMethod"
            color="neutral"
            :label="t(entry.acquisitionMethod === 'map' ? 'scan.viewOem' : 'scan.viewIntel')"
            size="xs"
            target="_blank"
            :to="entry.acquisitionMethod === 'map' ? entry.oemUrl : entry.intelUrl"
            trailing-icon="i-lucide-external-link"
            variant="outline"
          />
        </dd>
      </div>
      <div class="space-y-1">
        <dt class="text-muted">{{ t('scan.collectionStatus') }}</dt>
        <dd class="space-y-2">
          <UBadge :color="entry.scans.length ? 'success' : 'neutral'" size="lg" variant="soft">
            {{ entry.scans.length ? t('scan.collected') : t('scan.notCollected') }}
          </UBadge>
          <div
            v-for="scan in entry.scans"
            :key="scan.scannedItemId"
            class="flex items-center gap-2 rounded-md bg-elevated p-2"
          >
            <div class="min-w-0 flex-1">
              <p>
                #{{ scan.scannedItemId }}
                {{ scan.manualChoice.kind === 'manual' ? t('scan.manual') : t('scan.automatic') }}
              </p>
              <p class="truncate text-muted" :title="scan.ocrResult || t('scan.noTextShort')">
                {{ t('scan.ocrValue', { text: scan.ocrResult || t('scan.noTextShort') }) }}
              </p>
            </div>
            <UButton
              color="neutral"
              :label="t('scan.locateScan')"
              size="xs"
              trailing-icon="i-lucide-arrow-right"
              variant="outline"
              @click="locateScan(scan.scannedItemId)"
            />
          </div>
        </dd>
      </div>
    </dl>
  </UCard>
</template>
