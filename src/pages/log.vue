<script setup lang="ts">
import { clearLogs, filteredLogLines, logLevelFilter } from '@/lib/logState';
import type { LogLevel } from '@/lib/tauri';
import { openLogDir } from '@/lib/tauri';
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue';

function openLogDirectory(): void {
  void openLogDir();
}

/** UTextarea 组件实例暴露的 textarea 元素。 */
interface TextareaInstance {
  textareaRef: HTMLTextAreaElement | null;
}

const textarea = useTemplateRef<TextareaInstance>('textarea');

/** 日志等级过滤选项（显示该等级及以上）。 */
const levelOptions: { label: string; value: LogLevel }[] = [
  { label: 'TRACE', value: 'TRACE' },
  { label: 'DEBUG', value: 'DEBUG' },
  { label: 'INFO', value: 'INFO' },
  { label: 'WARN', value: 'WARN' },
  { label: 'ERROR', value: 'ERROR' },
];

const text = computed(() =>
  filteredLogLines.value.map((entry) => `${entry.time} ${entry.level} ${entry.message}`).join('\n'),
);

function getTextarea(): HTMLTextAreaElement | null {
  return textarea.value?.textareaRef ?? null;
}

/** 是否已滚动到底部（用于判断是否跟随新日志自动滚动）。 */
function isAtBottom(): boolean {
  const el = getTextarea();
  if (!el) {
    return true;
  }
  return el.scrollTop + el.clientHeight >= el.scrollHeight - 4;
}

function scrollToBottom(): void {
  const el = getTextarea();
  if (el) {
    el.scrollTo({ top: el.scrollHeight });
  }
}

// 挂载时滚动到底部，直接展示最新日志
onMounted(() => {
  nextTick(scrollToBottom);
});

// 新日志（或过滤等级变化）到来时，若用户已在底部则自动跟随滚动
watch(filteredLogLines, () => {
  if (isAtBottom()) {
    requestAnimationFrame(scrollToBottom);
  }
});
</script>

<template>
  <UContainer class="h-[calc(100dvh-var(--ui-header-height))] py-6">
    <div class="flex h-full flex-col gap-6">
      <div class="flex flex-wrap gap-2">
        <UButton
          icon="i-lucide-folder-open"
          label="打开日志文件目录"
          size="lg"
          @click="openLogDirectory"
        />
        <UButton
          color="error"
          icon="i-lucide-trash-2"
          label="清空日志"
          size="lg"
          variant="outline"
          @click="clearLogs"
        />
      </div>

      <div class="flex h-full min-h-0 flex-col gap-2">
        <div class="flex items-center justify-between gap-2">
          <h2 class="text-sm font-medium">实时日志</h2>
          <USelect v-model="logLevelFilter" class="w-32" :items="levelOptions" size="sm" />
        </div>
        <UTextarea
          ref="textarea"
          class="min-h-0 flex-1"
          color="neutral"
          :model-value="text"
          placeholder="暂无日志…"
          readonly
          size="sm"
          :ui="{ base: 'h-full resize-none font-mono text-xs' }"
          variant="subtle"
        />
      </div>
    </div>
  </UContainer>
</template>
