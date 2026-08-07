<script setup lang="ts">
import { CollectType, ScanResultCardProps, ScanResultStatus } from '@/types/scanResult';
import { appStatus } from '@/utils/app/appStatus';
import { prtsData } from '@/utils/app/prtsData';
import { clearScanResults, scanResults } from '@/utils/app/scanResults';
import { startScan, stopScan } from '@/utils/tauri';
import { computed, ref } from 'vue';

function toggleScan() {
  return appStatus.value.running ? stopScan() : startScan();
}

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

/** 导出扫描结果到地图集。 */
function exportToOem() {
  // TODO: 待实现导出逻辑
}

const hideCollected = ref(false);
const hideUncollectible = ref(false);

const filteredScanResults = computed<ScanResultCardProps[]>(() => {
  const result: ScanResultCardProps[] = [];
  for (const { status, category, subCategory, correctedTitle, image } of scanResults.value) {
    if (status === 'success') {
      continue;
    }
    result.push({
      collectType: statusToCollectType(status),
      category,
      subCategory,
      imageUrl: image,
      title: correctedTitle ?? '',
    });
  }

  for (const { categoryId, id, title, type } of Object.values(prtsData.value?.allItems ?? {})) {
    const maybeScanResult = scanResults.value.find((r) => r.itemIds.includes(id));
    if (maybeScanResult !== undefined) {
      const { status, category, subCategory, correctedTitle, image } = maybeScanResult;
      if (hideCollected.value && status === 'success') {
        continue;
      }
      result.push({
        collectType: statusToCollectType(status),
        category,
        subCategory,
        imageUrl: image,
        title: correctedTitle ?? title,
      });
    } else {
      result.push({
        collectType: CollectType.NotCollected,
        category: type,
        subCategory: categoryId,
        imageUrl: null,
        title,
      });
    }
  }
  return result;
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

      <div class="flex flex-1 flex-col gap-2 overflow-y-hidden">
        <div class="flex flex-0 flex-wrap items-center justify-between">
          <p class="text-sm font-medium">扫描结果</p>
          <div class="flex flex-wrap gap-4">
            <UCheckbox v-model="hideCollected" color="info" label="隐藏已收集" />
            <UCheckbox v-model="hideUncollectible" color="info" label="隐藏任务获取的档案" />
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <span class="ml-auto text-xs text-muted">共 {{ scanResults.length }} 条</span>
            <UButton
              color="error"
              icon="i-lucide-trash-2"
              label="清空"
              size="xs"
              variant="ghost"
              @click="clearScanResults()"
            />
          </div>
        </div>

        <div class="flex-1 scrollbar-gutter-stable space-y-2 overflow-y-auto p-1">
          <template v-for="(scanResult, index) in filteredScanResults" :key="index">
            <ScanResultCard v-bind="scanResult" />
          </template>
        </div>
      </div>
    </div>
  </UContainer>
</template>
