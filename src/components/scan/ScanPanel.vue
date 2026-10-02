<script setup lang="ts">
import {
  ArchiveScanCardStatus,
  ArchiveScanResultCardProps,
} from '@/features/archiveScan/types/archiveScanResultCard';
import type { ScannedItemStatus } from '@/features/archiveScan/types/scannedItem';
import { useAutomationTask, type CommandResult } from '@/features/automation/useAutomationTask';
import { getAcquisitionMethod } from '@/features/gameData/archiveContract';
import { exportToOem } from '@/features/archiveScan/exportOem';
import { prtsData } from '@/features/gameData/prtsData';
import {
  clearScannedItems,
  correctScannedItem,
  scannedItems,
  scanError,
  type ScannedItemRecord,
} from '@/features/archiveScan/scannedItems';
import { deriveArchiveCollection } from '@/features/archiveScan/collection';
import { archiveScanWorkerType } from '@/features/archiveScan/workerType';
import { computed, ref, watch, type ComputedRef, type DeepReadonly } from 'vue';

const { phase, isActive, canStart, canStop, tryStart, tryStop } = useAutomationTask('archiveScan');

function toggleScan(): Promise<CommandResult> {
  return canStart.value ? tryStart({ workerType: archiveScanWorkerType.value }) : tryStop();
}

const scanButtonLabel = computed<string>(() => {
  if (phase.value === 'stopping') return '正在停止扫描';
  if (phase.value === 'blocked') return '其他自动化任务正在运行';
  if (phase.value === 'running') {
    return '停止扫描（引号键）';
  }
  return archiveScanWorkerType.value === 'simulation' ? '开始模拟扫描' : '开始扫描（引号键）';
});

/** 用户是否手动关闭了扫描失败提示（失败原因变化时自动恢复显示） */
const scanErrorDismissed = ref(false);

/** 是否展示扫描失败提示（存在失败原因且未被手动关闭） */
const showScanError = computed(() => scanError.value !== null && !scanErrorDismissed.value);

/** 当前扫描失败原因（无失败时为 undefined，用于提示文案） */
const scanErrorMessage = computed(() => scanError.value ?? undefined);

// 失败原因变化（含重新失败）时恢复显示提示
watch(scanError, () => {
  scanErrorDismissed.value = false;
});

function toCardStatus(status: ScannedItemStatus): ArchiveScanCardStatus {
  switch (status) {
    case 'success':
      return ArchiveScanCardStatus.Matched;
    case 'unrecognized':
      return ArchiveScanCardStatus.Unrecognized;
    case 'failed':
      return ArchiveScanCardStatus.OcrFailed;
  }
}

const hideCollected = ref(false);
const hideNotObtainableInOverworld = ref(false);

/** 档案是否可在大世界中获取：仅获取方式为地图交互点位（method = 'map'）的档案可在世界内直接收集。 */
function isNotObtainableInOverworld(archiveId: string | null): boolean {
  return archiveId !== null && getAcquisitionMethod(archiveId) !== 'map';
}

const filteredResultCards: ComputedRef<ArchiveScanResultCardProps[]> = computed(
  (): ArchiveScanResultCardProps[] => {
    const result: ArchiveScanResultCardProps[] = [];
    for (const scannedItem of scannedItems.value) {
      if (scannedItem.status === 'success') {
        continue;
      }
      result.push({
        status: toCardStatus(scannedItem.status),
        category: scannedItem.foundInCategory,
        subCategory: scannedItem.foundInSubCategory,
        imageUrl: scannedItem.image,
        title: scannedItem.correctedTitle ?? scannedItem.ocrResult,
        archiveId: scannedItem.correctedMatchItemIds[0] ?? null,
        scannedItemId: scannedItem.scannedItemId,
      });
    }

    for (const { categoryId, id, title, type } of Object.values(prtsData.value?.allItems ?? {})) {
      // 隐藏无法在大世界中获取的档案（获取方式非地图交互点位）
      if (hideNotObtainableInOverworld.value && isNotObtainableInOverworld(id)) {
        continue;
      }
      const maybeScannedItem: DeepReadonly<ScannedItemRecord> | undefined = scannedItems.value.find(
        (item: DeepReadonly<ScannedItemRecord>): boolean => item.correctedMatchItemIds.includes(id),
      );
      if (maybeScannedItem !== undefined) {
        if (hideCollected.value && maybeScannedItem.status === 'success') {
          continue;
        }
        result.push({
          status: toCardStatus(maybeScannedItem.status),
          category: maybeScannedItem.foundInCategory,
          subCategory: maybeScannedItem.foundInSubCategory,
          imageUrl: maybeScannedItem.image,
          title: maybeScannedItem.correctedTitle ?? title,
          archiveId: id,
          scannedItemId: maybeScannedItem.scannedItemId,
        });
      } else {
        result.push({
          status: ArchiveScanCardStatus.NotMatched,
          category: type,
          subCategory: categoryId,
          imageUrl: null,
          title,
          archiveId: id,
          scannedItemId: null,
        });
      }
    }
    return result;
  },
);
/**
 * 扫描结果统计（与导出到地图集口径一致：同一小分类下同标题档案只要有一个已收集，该组全部视为已收集）。
 * 已收集 / 未收集为档案数，识别错误为扫描失败（failed / unrecognized）条数。
 */
interface ScanSummary {
  notSuccessCount: number;
  notCollectedCount: number;
  collectedCount: number;
}

const summary: ComputedRef<ScanSummary> = computed((): ScanSummary => {
  const collection = deriveArchiveCollection(prtsData.value?.allItems ?? {}, scannedItems.value);
  const notSuccessCount: number = scannedItems.value.filter(
    (item: DeepReadonly<ScannedItemRecord>): boolean => item.status !== 'success',
  ).length;
  return {
    notSuccessCount,
    notCollectedCount: collection.notCollectedIds.length,
    collectedCount: collection.collectedIds.length,
  };
});
</script>

<template>
  <UContainer class="h-full py-4">
    <div class="flex h-full flex-col gap-4">
      <div class="flex flex-wrap gap-2">
        <UButton
          :color="
            isActive ? 'error' : archiveScanWorkerType === 'simulation' ? 'warning' : 'success'
          "
          :disabled="!canStart && !canStop"
          :icon="isActive ? 'i-lucide-square' : 'i-lucide-play'"
          :label="scanButtonLabel"
          @click="toggleScan"
        />
        <UButton class="ms-auto" icon="i-lucide-map" label="导出到地图集" @click="exportToOem" />
      </div>

      <UAlert
        v-if="showScanError"
        close
        color="error"
        :description="scanErrorMessage"
        icon="i-lucide-circle-alert"
        orientation="horizontal"
        title="扫描失败"
        variant="outline"
        @update:open="scanErrorDismissed = true"
      >
        <template #actions>
          <UButton
            color="info"
            icon="i-lucide-scroll-text"
            label="前往日志页查看详情"
            size="sm"
            to="/log"
            variant="outline"
          />
        </template>
      </UAlert>

      <div class="flex flex-1 flex-col gap-2 overflow-y-hidden">
        <div class="flex flex-0 flex-wrap items-center justify-between">
          <div class="flex items-center gap-2">
            <p class="text-sm font-medium">扫描结果</p>
            <div class="flex items-center gap-1.5">
              <span class="rounded bg-error/10 px-1.5 py-0.5 text-xs text-error">
                识别错误 {{ summary.notSuccessCount }}
              </span>
              <span class="rounded bg-elevated px-1.5 py-0.5 text-xs text-muted">
                未收集 {{ summary.notCollectedCount }}
              </span>
              <span class="rounded bg-success/10 px-1.5 py-0.5 text-xs text-success">
                已收集 {{ summary.collectedCount }}
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
    </div>
  </UContainer>
</template>
