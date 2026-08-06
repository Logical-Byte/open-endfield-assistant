<script setup lang="ts">
import { useLogState } from '@/composables/app/useLogState';
import { useScrollToBottom } from '@/composables/useScrollToBottom';
import { openLogDir } from '@/utils/tauri';
import { useTemplateRef } from 'vue';

const { clearLogs, filteredLogLines, logLevelFilter, levelOptions } = useLogState();

/** 把 ISO 8601 时间字符串格式化为 `MM-dd HH:MM:SS`（解析失败时原样返回）。 */
function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  const pad = (value: number, length = 2): string => String(value).padStart(length, '0');
  return `${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

const logContainerRef = useTemplateRef('logContainerRef');

useScrollToBottom(logContainerRef, filteredLogLines);
</script>

<template>
  <UContainer class="flex h-full flex-col gap-4 py-4">
    <div class="flex flex-wrap gap-2">
      <UButton
        class="mr-auto"
        icon="i-lucide-folder-open"
        label="打开日志文件目录"
        @click="openLogDir"
      />
      <USelect v-model="logLevelFilter" class="w-32" :items="levelOptions" />
      <UButton
        color="error"
        icon="i-lucide-trash-2"
        label="清空日志"
        variant="outline"
        @click="clearLogs"
      />
    </div>

    <UCard class="min-h-0 flex-1" :ui="{ body: 'h-full p-0!' }">
      <div ref="logContainerRef" class="h-full scrollbar-gutter-stable overflow-y-scroll px-6 py-4">
        <p v-if="filteredLogLines.length === 0" class="font-mono text-muted">暂无日志</p>
        <p v-for="(line, index) in filteredLogLines" :key="index" class="font-mono leading-normal">
          {{ formatTime(line.time) }} [{{ line.level }}] {{ line.message }}
        </p>
      </div>
    </UCard>
  </UContainer>
</template>
