<script setup lang="ts">
import { useAppState } from '@/composables/app/useAppState';
import { useScanResults } from '@/composables/app/useScanResults';
import { scanSingle, startScan, stopScan } from '@/utils/tauri';

const { appStatus } = useAppState();
const { scanResults, pushMockScanResult, clearScanResults } = useScanResults();

async function toggleScan() {
  return appStatus.value.running ? stopScan() : startScan();
}

/** 导出扫描结果到地图集。 */
function exportToOem() {
  // TODO: 待实现导出逻辑
}
</script>

<template>
  <UContainer class="h-full py-4">
    <div class="flex h-full flex-col gap-4">
      <div class="flex flex-wrap gap-2">
        <UButton
          :color="appStatus.running ? 'error' : 'success'"
          :icon="appStatus.running ? 'i-lucide-square' : 'i-lucide-play'"
          :label="appStatus.running ? '停止扫描' : '启动扫描'"
          @click="toggleScan"
        />
        <UButton
          color="neutral"
          :disabled="appStatus.running"
          icon="i-lucide-scan"
          label="单次扫描"
          variant="outline"
          @click="scanSingle"
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
        <div class="flex flex-0 flex-wrap items-center gap-3">
          <h2 class="mr-auto text-sm font-medium">扫描结果</h2>
          <span class="text-xs text-muted">共 {{ scanResults.length }} 条</span>
          <UButton
            color="neutral"
            icon="i-lucide-flask-conical"
            label="模拟数据"
            size="xs"
            variant="ghost"
            @click="pushMockScanResult()"
          />
          <UButton
            color="neutral"
            icon="i-lucide-trash-2"
            label="清空"
            size="xs"
            variant="ghost"
            @click="clearScanResults()"
          />
        </div>

        <div class="flex-1 scrollbar-gutter-stable space-y-2 overflow-y-auto p-1">
          <ScanResultCard
            v-for="result in scanResults"
            :key="result.index"
            v-bind="result"
            v-model:ocr_text="result.ocr_result"
          />
        </div>
      </div>
    </div>
  </UContainer>
</template>
