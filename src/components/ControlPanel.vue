<script setup lang="ts">
import { useAppState } from '@/composables/app/useAppState';
import { scanSingle, startScan, stopScan } from '@/utils/tauri';

const { appStatus } = useAppState();

async function handleToggle() {
  return appStatus.value.running ? await stopScan() : await startScan();
}

/** 导出扫描结果到地图集。 */
function handleExportToAtlas() {
  // TODO: 待实现导出逻辑
}
</script>

<template>
  <div class="flex flex-wrap gap-2">
    <UButton
      :color="appStatus.running ? 'error' : 'success'"
      :icon="appStatus.running ? 'i-lucide-square' : 'i-lucide-play'"
      :label="appStatus.running ? '停止扫描' : '启动扫描'"
      size="lg"
      @click="handleToggle"
    />
    <UButton
      color="neutral"
      :disabled="appStatus.running"
      icon="i-lucide-scan"
      label="单次扫描"
      size="lg"
      variant="outline"
      @click="scanSingle"
    />
    <UButton
      class="ms-auto"
      color="neutral"
      icon="i-lucide-map"
      label="导出到地图集"
      size="lg"
      variant="outline"
      @click="handleExportToAtlas"
    />
  </div>
</template>
