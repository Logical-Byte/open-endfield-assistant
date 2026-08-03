<script setup lang="ts">
import { clearScanResults, initScanResults, scanResults } from '@/lib/scanResults';

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

    <UCard v-for="result in scanResults" :key="result.index">
      <template #header>
        <div class="flex flex-wrap items-center gap-3">
          <span class="text-sm font-semibold">#{{ result.index }}</span>
          <UBadge
            :color="result.status === 'success' ? 'success' : 'error'"
            :label="result.status === 'success' ? '成功' : '失败'"
            variant="subtle"
          />
          <UBadge color="neutral" :label="result.category" variant="outline" />
        </div>
      </template>

      <div class="flex flex-col gap-4 sm:flex-row">
        <img
          alt="档案详情截图"
          class="w-full shrink-0 rounded-md bg-elevated ring-1 ring-default sm:w-72"
          :src="result.image"
        />
        <div class="flex min-w-0 flex-1 flex-col gap-2">
          <span class="text-xs font-medium text-muted">OCR 识别结果（可编辑）</span>
          <UTextarea
            v-model="result.ocr_result"
            autoresize
            color="neutral"
            :maxrows="8"
            placeholder="识别结果为空…"
            size="sm"
            variant="subtle"
          />
        </div>
      </div>
    </UCard>
  </div>
</template>
