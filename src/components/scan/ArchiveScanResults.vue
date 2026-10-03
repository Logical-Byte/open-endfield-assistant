<script setup lang="ts">
import type { ArchiveScanResultCardProps } from '@/features/archiveScan/types/archiveScanResultCard';
import {
  deriveArchiveScanView,
  filterArchiveScanCards,
  type ArchiveScanView,
} from '@/features/archiveScan/resultView';
import {
  clearScannedItems,
  correctScannedItem,
  scannedItems,
} from '@/features/archiveScan/scannedItems';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import { methodByArchiveId } from '@/features/gameData/archiveContract';
import { prtsData } from '@/features/gameData/prtsData';
import { computed, ref, type ComputedRef, type Ref } from 'vue';

const { isActive } = useAutomationTask('archiveScan');

const hideCollected: Ref<boolean> = ref(false);
const hideNotObtainableInOverworld: Ref<boolean> = ref(false);

const resultView: ComputedRef<ArchiveScanView> = computed((): ArchiveScanView =>
  deriveArchiveScanView(prtsData.value, methodByArchiveId.value, scannedItems.value),
);
const filteredResultCards: ComputedRef<ArchiveScanResultCardProps[]> = computed(
  (): ArchiveScanResultCardProps[] =>
    filterArchiveScanCards(resultView.value.cards, {
      hideCollected: hideCollected.value,
      hideNotObtainableInOverworld: hideNotObtainableInOverworld.value,
    }),
);
</script>

<template>
  <div class="flex flex-1 flex-col gap-2 overflow-y-hidden">
    <div class="flex flex-0 flex-wrap items-center justify-between">
      <div class="flex items-center gap-2">
        <p class="text-sm font-medium">扫描结果</p>
        <div class="flex items-center gap-1.5">
          <span class="rounded bg-error/10 px-1.5 py-0.5 text-xs text-error">
            识别错误 {{ resultView.summary.notSuccessCount }}
          </span>
          <span class="rounded bg-elevated px-1.5 py-0.5 text-xs text-muted">
            未收集 {{ resultView.summary.notCollectedCount }}
          </span>
          <span class="rounded bg-success/10 px-1.5 py-0.5 text-xs text-success">
            已收集 {{ resultView.summary.collectedCount }}
          </span>
        </div>
      </div>
      <div class="flex flex-wrap gap-4">
        <UCheckbox v-model="hideCollected" color="info" label="隐藏已收集" />
        <UCheckbox
          v-model="hideNotObtainableInOverworld"
          color="info"
          label="隐藏无法在大世界中获取的档案"
        />
      </div>

      <UButton
        color="error"
        :disabled="isActive"
        icon="i-lucide-trash-2"
        label="清空"
        size="xs"
        variant="ghost"
        @click="clearScannedItems()"
      />
    </div>

    <UScrollArea
      v-slot="{ item }"
      class="flex-1 scrollbar-gutter-stable p-1"
      :items="filteredResultCards"
      :virtualize="{
        estimateSize: 56,
        skipMeasurement: true,
        overscan: 8,
      }"
    >
      <ArchiveScanResultCard
        v-bind="item"
        @correct="item.scannedItemId !== null && correctScannedItem(item.scannedItemId, $event)"
      />
    </UScrollArea>
  </div>
</template>
