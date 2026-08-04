<script setup lang="ts">
import { running } from '@/lib/appState';
import { scanSingle, startScan, stopScan } from '@/lib/tauri';

async function handleToggle() {
  return running.value ? await stopScan() : await startScan();
}

/** 导出扫描结果到地图集。 */
function handleExportToAtlas() {
  // TODO: 待实现导出逻辑
}
</script>

<template>
  <div class="flex flex-wrap gap-2">
    <UButton
      :color="running ? 'error' : 'success'"
      :icon="running ? 'i-lucide-square' : 'i-lucide-play'"
      :label="running ? '停止扫描' : '启动扫描'"
      size="lg"
      @click="handleToggle"
    />
    <UButton
      color="neutral"
      :disabled="running"
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
