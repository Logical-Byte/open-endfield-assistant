<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import {
  useAutomationTask,
  type CommandResult,
  type AutomationTask,
} from '@/features/automation/useAutomationTask';
import { archiveScanWorkerType } from '@/features/archiveScan/workerType';
import { computed, type ComputedRef } from 'vue';

const { t } = useAppI18n();

const { phase, isActive, canStart, canStop, tryStart, tryStop }: AutomationTask<'archiveScan'> =
  useAutomationTask('archiveScan');

function toggleScan(): Promise<CommandResult> {
  return canStart.value ? tryStart({ workerType: archiveScanWorkerType.value }) : tryStop();
}

const scanButtonLabel: ComputedRef<string> = computed((): string => {
  if (phase.value === 'stopping') return t('scan.stopping');
  if (phase.value === 'blocked') return t('scan.blocked');
  if (phase.value === 'running') {
    return t('scan.stop');
  }
  return archiveScanWorkerType.value === 'simulation' ? t('scan.startSimulation') : t('scan.start');
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
