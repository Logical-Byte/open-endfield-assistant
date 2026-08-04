<script setup lang="ts">
import { initScanResults } from '@/lib/scanResults';

await initScanResults();
</script>

<template>
  <UContainer>
    <UPage>
      <UPageBody>
        <ControlPanel />
        <ScanResultList />
      </UPageBody>
    </UPage>
  </UContainer>
</template>
