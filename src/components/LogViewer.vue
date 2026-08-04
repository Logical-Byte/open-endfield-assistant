<script setup lang="ts">
import { onLog } from '@/lib/tauri';
import { onMounted, onUnmounted, ref } from 'vue';

const lines = ref<string[]>([]);
const container = ref<HTMLElement | null>(null);
let unlisten: (() => void) | null = null;

onMounted(async () => {
  unlisten = await onLog((line) => {
    lines.value.push(line);
    // 自动滚动到底部
    requestAnimationFrame(() => {
      container.value?.scrollTo({ top: container.value.scrollHeight });
    });
  });
});

onUnmounted(() => {
  unlisten?.();
});

function clear(): void {
  lines.value = [];
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
      <p v-if="lines.length === 0" class="text-muted">暂无日志…</p>
      <p v-for="(line, i) in lines" :key="i" class="text-toned">{{ line }}</p>
    </div>
  </div>
</template>
