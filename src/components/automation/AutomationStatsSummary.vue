<script setup lang="ts">
import type { CaptureSummary } from '@/types/automationStats';
import { computed } from 'vue';

const props = defineProps<{
  summary: CaptureSummary;
}>();

const emit = defineEmits<{
  close: [];
}>();

// CaptureSummary 保留完整诊断字段；用户摘要有意不展示 moveMouseToSafePosition 和 sleep。
const elapsedSeconds = computed(() => (props.summary.elapsedMicros / 1_000_000).toFixed(1));
</script>

<template>
  <section
    aria-label="本次自动化统计"
    class="flex items-center gap-1 rounded-md bg-elevated/60 py-1.5 ps-3 pe-1.5 text-xs text-muted"
  >
    <div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1">
      <span class="inline-flex items-center gap-1.5 font-medium text-highlighted">
        <UIcon class="size-3.5 shrink-0 text-success" name="i-lucide-gauge" />
        本次自动化统计
      </span>
      <span class="hidden h-3 w-px bg-border sm:block" />
      <span>{{ elapsedSeconds }} 秒</span>
      <span>截图 {{ summary.calls.screenshot }} 次</span>
      <span>点击 {{ summary.calls.click }} 次</span>
      <span>按键 {{ summary.calls.pressKey }} 次</span>
      <span>OCR {{ summary.calls.recognizeText }} 次</span>
      <span>模板匹配 {{ summary.calls.findTemplate }} 次</span>
    </div>
    <UButton
      aria-label="关闭本次自动化统计"
      class="shrink-0"
      color="neutral"
      icon="i-lucide-x"
      size="xs"
      variant="ghost"
      @click="emit('close')"
    />
  </section>
</template>
