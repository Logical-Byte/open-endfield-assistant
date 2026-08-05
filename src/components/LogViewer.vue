<script setup lang="ts">
import { clearLogs, logLines } from '@/lib/logState';
import { nextTick, onMounted, ref, watch } from 'vue';

const container = ref<HTMLElement | null>(null);

/** 是否已滚动到底部（用于判断是否跟随新日志自动滚动）。 */
function isAtBottom(): boolean {
  const el = container.value;
  if (!el) {
    return true;
  }
  return el.scrollTop + el.clientHeight >= el.scrollHeight - 4;
}

function scrollToBottom(): void {
  container.value?.scrollTo({ top: container.value.scrollHeight });
}

// 挂载时滚动到底部，直接展示最新日志
onMounted(() => {
  nextTick(scrollToBottom);
});

// 新日志到来时，若用户已在底部则自动跟随滚动
watch(
  logLines,
  () => {
    if (isAtBottom()) {
      requestAnimationFrame(scrollToBottom);
    }
  },
  { deep: true },
);

function clear(): void {
  clearLogs();
}

defineExpose({ clear });
</script>

<template>
  <div class="flex flex-col gap-2">
    <h2 class="text-sm font-medium">实时日志</h2>
    <div
      ref="container"
      class="h-64 overflow-y-auto rounded-md bg-default p-3 font-mono text-xs leading-relaxed"
    >
      <p v-if="logLines.length === 0" class="text-muted">暂无日志…</p>
      <p v-for="(line, i) in logLines" :key="i" class="text-toned">{{ line }}</p>
    </div>
  </div>
</template>
