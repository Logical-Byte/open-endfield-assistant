<script setup lang="ts">
import { shallowRef } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import type { ArchiveScanSource } from '@/features/archiveScan/useArchiveScanResults';

const previewSource = shallowRef<ArchiveScanSource>();
const showPreviewControls = import.meta.env.DEV && !isTauri();

async function fillPreview(): Promise<void> {
  const { createScanPreviewSource } = await import('@/features/archiveScan/scanPreview');
  previewSource.value = createScanPreviewSource();
}
</script>

<template>
  <div class="relative h-full w-full">
    <ScanPanel :key="previewSource ? 'preview' : 'live'" :source="previewSource" />
    <ScanGuide v-if="!previewSource" />
    <div
      v-if="showPreviewControls"
      class="absolute bottom-10 left-3 z-30 flex items-center gap-2 rounded-lg border border-default bg-elevated p-2 shadow-lg"
    >
      <span v-if="previewSource" class="text-xs text-muted">示例数据</span>
      <UButton
        color="neutral"
        icon="i-lucide-flask-conical"
        size="xs"
        variant="outline"
        @click="fillPreview"
      >
        {{ previewSource ? '重新填充' : '填充示例数据' }}
      </UButton>
      <UButton
        v-if="previewSource"
        color="neutral"
        size="xs"
        variant="ghost"
        @click="previewSource = undefined"
        >退出示例</UButton
      >
    </div>
  </div>
</template>
