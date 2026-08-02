<script setup lang="ts">
import { running } from '@/lib/appState';
import { quitApp, scanSingle, startScan, stopScan } from '@/lib/tauri';

async function handleToggle() {
  running.value = running.value ? (await stopScan()).running : (await startScan()).running;
}

async function handleScanSingle() {
  await scanSingle();
}

async function handleQuit() {
  await quitApp();
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
      @click="handleScanSingle"
    />
    <UButton
      color="neutral"
      icon="i-lucide-power"
      label="退出程序"
      variant="ghost"
      @click="handleQuit"
    />
  </div>
</template>
