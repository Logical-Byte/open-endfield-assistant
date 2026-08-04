<script setup lang="ts">
import {
  clearScanResults,
  initScanResults,
  pushMockScanResult,
  scanResults,
} from '@/lib/scanResults';

initScanResults();
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-medium">扫描结果</h2>
      <div class="flex items-center gap-3">
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
    </div>

    <p v-if="scanResults.length === 0" class="text-sm text-muted">
      暂无扫描结果，启动扫描后识别到的档案将在此处以卡片形式展示。
    </p>

    <ScanResultCard
      v-for="result in scanResults"
      :key="result.index"
      v-bind="result"
      v-model:ocr_text="result.ocr_result"
    />
  </div>
</template>
