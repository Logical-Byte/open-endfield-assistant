<script setup lang="ts">
import { initAppState } from '@/lib/appState';
import { initScanResults } from '@/lib/scanResults';

initAppState();
initScanResults();
</script>

<template>
  <UContainer>
    <UPage>
      <UPageBody>
        <div class="flex flex-col gap-6">
          <div>
            <h1 class="text-2xl font-semibold">明日方舟：终末地 档案库扫描助手</h1>
            <p class="mt-1 text-muted">
              通过快捷键或下方按钮控制档案库扫描；扫描结果将实时展示在下方卡片中，识别文本可手动修正。
            </p>
          </div>

          <StatusCard />
          <ControlPanel />
          <ScanResultList />
        </div>
      </UPageBody>
    </UPage>
  </UContainer>
</template>
