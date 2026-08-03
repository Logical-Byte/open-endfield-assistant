<script setup lang="ts">
import type { ScanResult } from '@/lib/tauri';

const { index, status, category, image } = defineProps<ScanResult>();
const ocr_text = defineModel<string>('ocr_text');
</script>

<template>
  <UCard>
    <template #header>
      <div class="flex flex-wrap items-center gap-3">
        <span class="text-sm font-semibold">#{{ index }}</span>
        <UBadge
          :color="status === 'success' ? 'success' : 'error'"
          :label="status === 'success' ? '成功' : '失败'"
          variant="subtle"
        />
        <UBadge color="neutral" :label="category" variant="outline" />
      </div>
    </template>

    <div class="flex flex-col gap-4 sm:flex-row">
      <img
        alt="档案详情截图"
        class="w-full shrink-0 rounded-md bg-elevated ring-1 ring-default sm:w-72"
        :src="image"
      />
      <div class="flex min-w-0 flex-1 flex-col gap-2">
        <span class="text-xs font-medium text-muted">OCR 识别结果（可编辑）</span>
        <UTextarea
          v-model="ocr_text"
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
</template>
