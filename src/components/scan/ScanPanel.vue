<script setup lang="ts">
import { exportToOem } from '@/features/archiveScan/exportOem';
</script>

<template>
  <UContainer class="h-full py-4">
    <div class="flex h-full flex-col gap-4">
      <div class="flex flex-wrap gap-2">
        <ArchiveScanButton />
        <UButton class="ms-auto" icon="i-lucide-map" label="导出到地图集" @click="exportToOem" />
      </div>

      <ArchiveScanErrorAlert />
      <ArchiveScanResults />
    </div>
  </UContainer>
</template>
