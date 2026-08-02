<script setup lang="ts">
import { quitApp, scanSingle, startScan, stopScan } from '../lib/tauri'
import { running } from '../lib/appState'

async function handleToggle() {
  running.value = running.value
    ? (await stopScan()).running
    : (await startScan()).running
}

async function handleScanSingle() {
  await scanSingle()
}

async function handleQuit() {
  await quitApp()
}
</script>

<template>
  <div class="flex flex-wrap gap-2">
    <UButton
      :color="running ? 'error' : 'success'"
      :label="running ? '停止扫描' : '启动扫描'"
      :icon="running ? 'i-lucide-square' : 'i-lucide-play'"
      size="lg"
      @click="handleToggle"
    />
    <UButton
      label="单次扫描"
      icon="i-lucide-scan"
      size="lg"
      color="neutral"
      variant="outline"
      :disabled="running"
      @click="handleScanSingle"
    />
    <UButton
      label="退出程序"
      icon="i-lucide-power"
      color="neutral"
      variant="ghost"
      @click="handleQuit"
    />
  </div>
</template>
