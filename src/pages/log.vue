<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { useScrollToBottom } from '@/composables/useScrollToBottom';
import {
  clearLogs,
  filteredLogLines,
  levelOptions,
  levelLabels,
  logLevelFilter,
} from '@/features/log/log';
import { openLogDir } from '@/features/log/ipc';
import { useTemplateRef } from 'vue';

const { t, d } = useAppI18n();

/** 日期使用统一格式，无法解析的诊断时间保留原文。 */
function formatTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : d(date, 'dateTimeSeconds');
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
        :label="t('log.openDirectory')"
        @click="openLogDir"
      />
      <USelect
        v-model="logLevelFilter"
        :aria-label="t('log.levelFilter')"
        class="w-32"
        :items="levelOptions"
      />
      <UButton
        color="error"
        icon="i-lucide-trash-2"
        :label="t('log.clear')"
        variant="outline"
        @click="clearLogs"
      />
    </div>

    <UCard class="min-h-0 flex-1" :ui="{ body: 'h-full p-0!' }">
      <div ref="logContainerRef" class="h-full scrollbar-gutter-stable overflow-y-scroll px-6 py-4">
        <p v-if="filteredLogLines.length === 0" class="font-mono text-muted">
          {{ t('log.empty') }}
        </p>
        <pre
          v-for="({ time, level, message }, index) in filteredLogLines"
          :key="index"
          class="font-mono leading-normal"
        ><span class="text-dimmed">{{ formatTime(time) }}</span> <span
            :class="{
              'text-dimmed': level === 'TRACE' || level === 'DEBUG',
              'text-info': level === 'INFO',
              'text-warning': level === 'WARN',
              'text-error': level === 'ERROR',
            }"
            >{{ t(levelLabels[level]) }}</span
          > <span class="whitespace-pre-wrap wrap-break-word">{{ message }}</span></pre>
      </div>
    </UCard>
  </UContainer>
</template>
