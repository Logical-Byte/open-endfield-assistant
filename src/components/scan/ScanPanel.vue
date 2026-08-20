<script setup lang="ts">
import { CollectType, ScanResultCardProps, ScanResultStatus } from '@/types/scanResult';
import { appStatus } from '@/utils/app/appStatus';
import { getAcquisitionMethod } from '@/utils/app/archiveContract';
import { applyCorrection } from '@/utils/app/correction';
import { buildUploadData, exportToOem } from '@/utils/app/exportOem';
import { prtsData } from '@/utils/app/prtsData';
import { clearScanResults, scanResults } from '@/utils/app/scanResults';
import { startScan, stopScan } from '@/utils/tauri';
import { computed, ref, watch } from 'vue';

function toggleScan() {
  return appStatus.value.running ? stopScan() : startScan();
}

/** 用户是否手动关闭了扫描失败提示（失败原因变化时自动恢复显示） */
const scanErrorDismissed = ref(false);

/** 是否展示扫描失败提示（存在失败原因且未被手动关闭） */
const showScanError = computed(
  () => appStatus.value.scanError !== null && !scanErrorDismissed.value,
);

/** 当前扫描失败原因（无失败时为 undefined，用于提示文案） */
const scanErrorMessage = computed(() => appStatus.value.scanError ?? undefined);

// 失败原因变化（含重新失败）时恢复显示提示
watch(
  () => appStatus.value.scanError,
  () => {
    scanErrorDismissed.value = false;
  },
);

function statusToCollectType(status: ScanResultStatus): CollectType {
  switch (status) {
    case 'success':
      return CollectType.Collected;
    case 'unrecognized':
      return CollectType.Unrecognized;
    case 'failed':
      return CollectType.Failed;
  }
}

/** 应用人工纠错：更新对应扫描结果（标记为已收集或无法识别）。 */
function onCorrect(scanResultIndex: number | null, title: string): void {
  if (scanResultIndex === null) {
    return;
  }
  const scanResult = scanResults.value.find((result) => result.index === scanResultIndex);
  if (scanResult !== undefined) {
    applyCorrection(scanResult, title);
  }
}

const hideCollected = ref(false);
const hideNotObtainableInOverworld = ref(false);

/** 档案是否可在大世界中获取：仅获取方式为地图交互点位（method = 'map'）的档案可在世界内直接收集。 */
function isNotObtainableInOverworld(archiveId: string | null): boolean {
  return archiveId !== null && getAcquisitionMethod(archiveId) !== 'map';
}

const filteredScanResults = computed<ScanResultCardProps[]>(() => {
  const result: ScanResultCardProps[] = [];
  for (const {
    status,
    index,
    category,
    subCategory,
    correctedTitle,
    ocrResult,
    image,
    itemIds,
  } of scanResults.value) {
    if (status === 'success') {
      continue;
    }
    result.push({
      collectType: statusToCollectType(status),
      category,
      subCategory,
      imageUrl: image,
      title: correctedTitle ?? ocrResult,
      archiveId: itemIds[0] ?? null,
      scanResultIndex: index,
    });
  }

  for (const { categoryId, id, title, type } of Object.values(prtsData.value?.allItems ?? {})) {
    // 隐藏无法在大世界中获取的档案（获取方式非地图交互点位）
    if (hideNotObtainableInOverworld.value && isNotObtainableInOverworld(id)) {
      continue;
    }
    const maybeScanResult = scanResults.value.find((r) => r.itemIds.includes(id));
    if (maybeScanResult !== undefined) {
      const { status, index, category, subCategory, correctedTitle, image } = maybeScanResult;
      if (hideCollected.value && status === 'success') {
        continue;
      }
      result.push({
        collectType: statusToCollectType(status),
        category,
        subCategory,
        imageUrl: image,
        title: correctedTitle ?? title,
        archiveId: id,
        scanResultIndex: index,
      });
    } else {
      result.push({
        collectType: CollectType.NotCollected,
        category: type,
        subCategory: categoryId,
        imageUrl: null,
        title,
        archiveId: id,
        scanResultIndex: null,
      });
    }
  }
  return result;
});
/**
 * 扫描结果统计（与导出到地图集口径一致：重名档案只要有一个已收集，全部视为已收集）。
 * 已收集 / 未收集为档案数，识别错误为扫描失败（failed / unrecognized）条数。
 */
const summary = computed(() => {
  const { data } = buildUploadData();
  const error = scanResults.value.filter((result) => result.status !== 'success').length;
  return {
    error,
    notCollected: data.prtsAllItems.notCollected.length,
    collected: data.prtsAllItems.collected.length,
  };
});
</script>

<template>
  <UContainer class="h-full py-4">
    <div class="flex h-full flex-col gap-4">
      <div class="flex flex-wrap gap-2">
        <UButton
          :color="appStatus.running ? 'error' : 'success'"
          :icon="appStatus.running ? 'i-lucide-square' : 'i-lucide-play'"
          :label="appStatus.running ? '停止扫描' : '开始扫描'"
          @click="toggleScan"
        />
        <UButton
          class="ms-auto"
          color="neutral"
          icon="i-lucide-map"
          label="导出到地图集"
          variant="outline"
          @click="exportToOem"
        />
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
                识别错误 {{ summary.error }}
              </span>
              <span class="rounded bg-elevated px-1.5 py-0.5 text-xs text-muted">
                未收集 {{ summary.notCollected }}
              </span>
              <span class="rounded bg-success/10 px-1.5 py-0.5 text-xs text-success">
                已收集 {{ summary.collected }}
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
            icon="i-lucide-trash-2"
            label="清空"
            size="xs"
            variant="ghost"
            @click="clearScanResults()"
          />
        </div>

        <UScrollArea
          v-slot="{ item }"
          class="flex-1 scrollbar-gutter-stable p-1"
          :items="filteredScanResults"
          :virtualize="{
            estimateSize: 56,
            skipMeasurement: true,
            overscan: 8,
          }"
        >
          <ScanResultCard v-bind="item" @correct="onCorrect(item.scanResultIndex, $event)" />
        </UScrollArea>
      </div>
    </div>
  </UContainer>
</template>
