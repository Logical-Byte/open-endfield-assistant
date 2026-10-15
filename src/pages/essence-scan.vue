<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  essenceScanReport,
  simulateEssenceScanInBrowser,
  stopEssenceScanInBrowser,
} from '@/features/essenceScan/state';
import { getEssenceCatalog } from '@/features/gameData/essenceIpc';
import type { EssenceRecognitionIssue, EssenceTermObservation } from '@/shared/types/essenceScan';
import type { WorkerType } from '@/shared/types/automation';
import { useAppI18n } from '@/shared/i18n';
import { formatBackendError } from '@/shared/errors';
import localTerms from '../../resources/data/essence.json';

const { t } = useAppI18n();
const native = isTauri();
const task = useAutomationTask('essenceScan');
const pending = ref(false);
const error = ref('');
const termNames = ref<Record<string, string>>({});
const progress = computed(() => essenceScanReport.value?.progress);
const browserRunning = computed((): boolean => !!progress.value && progress.value.outcome === null);
const canStart = computed(
  (): boolean => !pending.value && (native ? task.canStart.value : !browserRunning.value),
);
const canStop = computed(
  (): boolean => !pending.value && (native ? task.canStop.value : browserRunning.value),
);
const outcomeText = computed((): string => {
  const value = progress.value;
  if (!value) return t('essenceScan.idle');
  if (value.outcome?.kind === 'failed')
    return formatBackendError({
      operation: 'automation',
      error: { scope: 'worker', ...value.outcome.error },
    });
  if (value.outcome) return t(`essenceScan.outcome.${value.outcome.kind}`);
  return t(`essenceScan.phase.${value.phase}`);
});

onMounted(async () => {
  try {
    const terms = native ? (await getEssenceCatalog()).terms : localTerms.terms;
    termNames.value = Object.fromEntries(terms.map((term) => [term.id, term.name]));
  } catch (cause) {
    error.value = String(cause);
  }
});

async function start(workerType: WorkerType): Promise<void> {
  pending.value = true;
  error.value = '';
  try {
    if (native) await task.tryStart({ workerType });
    else void simulateEssenceScanInBrowser();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    pending.value = false;
  }
}

async function stop(): Promise<void> {
  pending.value = true;
  try {
    if (native) await task.tryStop();
    else stopEssenceScanInBrowser();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    pending.value = false;
  }
}

function termText(term: Readonly<EssenceTermObservation>): string {
  const name = term.termId
    ? (termNames.value[term.termId] ?? term.termId)
    : t('essenceScan.unknown');
  return `${name} +${term.level ?? '?'}`;
}

function markerText(value: boolean | null, yes: string, no: string): string {
  return value === null ? t('essenceScan.unknown') : value ? yes : no;
}

function issueText(issue: Readonly<EssenceRecognitionIssue>): string {
  return t(
    `essenceScan.issue.${issue.kind}`,
    'position' in issue ? { position: issue.position } : {},
  );
}
</script>

<template>
  <div class="mx-auto max-w-5xl space-y-4 p-4">
    <h1 class="text-xl font-semibold">{{ t('essenceScan.title') }}</h1>
    <p class="text-sm text-muted">{{ t('essenceScan.instructions') }}</p>
    <div class="flex flex-wrap gap-2">
      <UButton
        :disabled="!native || !canStart"
        icon="i-lucide-scan-line"
        @click="start('production')"
        >{{ t('essenceScan.start') }}</UButton
      >
      <UButton
        color="neutral"
        :disabled="!canStart"
        variant="outline"
        @click="start('simulation')"
        >{{ t('essenceScan.simulate') }}</UButton
      >
      <UButton color="neutral" :disabled="!canStop" variant="outline" @click="stop">{{
        t('essenceScan.stop')
      }}</UButton>
    </div>
    <p v-if="!native || progress?.workerType === 'simulation'" class="text-sm text-muted">
      {{ t('essenceScan.simulationNotice') }}
    </p>
    <p v-if="error" class="text-error" role="alert">{{ error }}</p>
    <p class="text-sm" role="status">
      {{ outcomeText }}
      <span v-if="progress" class="ml-2">{{
        t('essenceScan.counts', {
          count: progress.scannedCount,
          incomplete: progress.incompleteCount,
        })
      }}</span>
    </p>
    <p v-if="!essenceScanReport?.records.length" class="text-muted">{{ t('essenceScan.empty') }}</p>
    <ol v-else class="divide-y divide-default">
      <li
        v-for="record in essenceScanReport.records"
        :key="`${record.runId}:${record.recordId}`"
        class="space-y-1 py-3"
      >
        <p class="text-sm text-muted">
          #{{ record.recordId }}
          {{ t('essenceScan.position', { row: record.row, column: record.column }) }}
        </p>
        <p>{{ record.terms.map(termText).join(' / ') }}</p>
        <p class="text-sm">
          {{ t(`essenceScan.rarity.${record.rarity}`) }} /
          {{ markerText(record.locked, t('essenceScan.locked'), t('essenceScan.unlocked')) }} /
          {{
            markerText(record.abandoned, t('essenceScan.abandoned'), t('essenceScan.notAbandoned'))
          }}
        </p>
        <p v-if="record.issues.length" class="text-sm text-warning">
          {{ record.issues.map(issueText).join('、') }}
        </p>
        <details v-if="record.image" class="text-sm">
          <summary>{{ t('essenceScan.image') }}</summary>
          <img
            :alt="t('essenceScan.image')"
            class="mt-2 max-w-full"
            height="303"
            :src="record.image"
            width="280"
          />
        </details>
      </li>
    </ol>
  </div>
</template>
