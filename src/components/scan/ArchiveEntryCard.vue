<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { useEvidencePopover } from './useEvidencePopover';
import type { ArchiveEntryView } from '@/features/archiveScan/resultView';
import type { ScannedItemId } from '@/features/archiveScan/types/scannedItem';
import ArchiveLabel from './ArchiveLabel.vue';
defineProps<{ entry: ArchiveEntryView; selected?: boolean }>();
const { t } = useAppI18n();

const emit = defineEmits<{ locateScan: [id: ScannedItemId]; clearHighlight: [] }>();
const { open, cancelClose, scheduleClose } = useEvidencePopover((): void => emit('clearHighlight'));
const acquisitionIcons: Record<string, string> = {
  map: 'i-lucide-map-pin',
  mission: 'i-lucide-flag',
  auto: 'i-lucide-unlock',
  shop: 'i-lucide-shopping-bag',
  invstgt: 'i-lucide-file-search',
};
</script>
<template>
  <article
    class="archive-row grid grid-cols-[minmax(0,1fr)_86px_112px] items-center gap-2 border-b border-default/60 px-3 py-2 transition-colors"
    :class="selected && 'bg-primary/20'"
  >
    <ArchiveLabel
      :category-label="entry.categoryLabel"
      class="text-base leading-6 group-data-[compact=true]/archive:col-span-full"
      :content-preview="entry.contentPreview"
      :title="entry.title"
    />
    <div
      class="group-data-[compact=true]/archive:col-start-1 group-data-[compact=true]/archive:row-start-2"
    >
      <UPopover
        v-if="entry.scans.length"
        v-model:open="open"
        :content="{ side: 'bottom', align: 'end', sideOffset: 5 }"
        mode="click"
        :ui="{ content: 'w-80 max-w-[calc(100vw-24px)] p-3' }"
      >
        <UButton
          :aria-expanded="open"
          color="success"
          :label="t('scan.matches', { count: entry.scans.length })"
          size="xs"
          trailing-icon="i-lucide-chevron-down"
          variant="subtle"
          @mouseenter="cancelClose"
          @mouseleave="scheduleClose"
        />
        <template #content
          ><div @mouseenter="cancelClose" @mouseleave="scheduleClose">
            <p class="mb-1 text-sm font-medium">{{ entry.title }}</p>
            <p class="mb-2 text-xs text-muted">
              {{ t('scan.evidenceCount', { count: entry.scans.length }) }}
            </p>
            <div class="max-h-64 space-y-2 overflow-auto">
              <div
                v-for="scan in entry.scans"
                :key="scan.scannedItemId"
                class="flex items-center gap-2 rounded border border-default p-2"
              >
                <div class="min-w-0 flex-1">
                  <p class="text-xs">
                    #{{ scan.scannedItemId }}
                    {{
                      scan.manualChoice.kind === 'manual' ? t('scan.manual') : t('scan.automatic')
                    }}
                  </p>
                  <p class="mt-1 truncate text-xs text-muted">
                    {{ t('scan.ocrValue', { text: scan.ocrResult || t('scan.noTextShort') }) }}
                  </p>
                </div>
                <UButton
                  color="neutral"
                  size="xs"
                  trailing-icon="i-lucide-arrow-right"
                  variant="outline"
                  @click="emit('locateScan', scan.scannedItemId)"
                  >{{ t('scan.locateScan') }}</UButton
                >
              </div>
            </div>
          </div></template
        >
      </UPopover>
      <UTooltip v-else :text="t('scan.noEvidence')">
        <span
          class="inline-flex h-6 items-center justify-center rounded border border-default bg-elevated px-2 text-xs text-toned"
          tabindex="0"
          >{{ t('scan.unmatched') }}</span
        >
      </UTooltip>
    </div>
    <div
      class="group-data-[compact=true]/archive:col-span-2 group-data-[compact=true]/archive:col-start-2 group-data-[compact=true]/archive:row-start-2 group-data-[compact=true]/archive:justify-self-end"
    >
      <UTooltip v-if="entry.acquisitionMethod" :text="t('scan.viewOem')">
        <UButton
          :aria-label="
            entry.acquisitionMethod === 'map' ? t('scan.viewLocation') : t(entry.acquisitionLabel)
          "
          class="justify-center"
          color="neutral"
          :label="
            entry.acquisitionMethod === 'map' ? t('scan.viewLocation') : t(entry.acquisitionLabel)
          "
          size="xs"
          target="_blank"
          :to="entry.acquisitionMethod === 'map' ? entry.oemUrl : entry.intelUrl"
          :trailing-icon="acquisitionIcons[entry.acquisitionMethod]"
          variant="outline"
        />
      </UTooltip>
      <span v-else class="flex h-7 items-center justify-center text-xs text-muted">{{
        t('scan.unknownAcquisition')
      }}</span>
    </div>
  </article>
</template>
