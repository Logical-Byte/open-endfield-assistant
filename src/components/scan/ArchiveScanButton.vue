<script setup lang="ts">
import {
  useAutomationTask,
  type CommandResult,
  type AutomationTask,
} from '@/features/automation/useAutomationTask';
import { archiveScanWorkerType } from '@/features/archiveScan/workerType';
import { computed, type ComputedRef } from 'vue';

const { phase, isActive, canStart, canStop, tryStart, tryStop }: AutomationTask<'archiveScan'> =
  useAutomationTask('archiveScan');

function toggleScan(): Promise<CommandResult> {
  return canStart.value ? tryStart({ workerType: archiveScanWorkerType.value }) : tryStop();
}

const scanButtonLabel: ComputedRef<string> = computed((): string => {
  if (phase.value === 'stopping') return '正在停止扫描';
  if (phase.value === 'blocked') return '其他自动化任务正在运行';
  if (phase.value === 'running') {
    return '停止扫描（引号键）';
  }
  return archiveScanWorkerType.value === 'simulation' ? '开始模拟扫描' : '开始扫描（引号键）';
});
</script>

<template>
  <UButton
    :color="isActive ? 'error' : archiveScanWorkerType === 'simulation' ? 'warning' : 'success'"
    :disabled="!canStart && !canStop"
    :icon="isActive ? 'i-lucide-square' : 'i-lucide-play'"
    :label="scanButtonLabel"
    @click="toggleScan"
  />
</template>
