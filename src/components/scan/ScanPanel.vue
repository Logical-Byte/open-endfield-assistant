<script setup lang="ts">
import type { ArchiveScanSource } from '@/features/archiveScan/useArchiveScanResults';
defineProps<{ source?: ArchiveScanSource }>();
</script>

<template>
  <UContainer class="h-full max-w-none p-0!">
    <div class="flex h-full min-h-0 flex-col gap-2">
      <ArchiveScanErrorAlert />
      <ArchiveScanResults :source="source" />
    </div>
  </UContainer>
</template>
