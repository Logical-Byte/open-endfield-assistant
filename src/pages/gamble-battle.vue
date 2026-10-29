<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useNow } from '@vueuse/core';
import WorkspaceCard from '@/components/layout/WorkspaceCard.vue';
import MatchedHandCard from '@/components/gamble-battle/MatchedHandCard.vue';
import GambleBattleGuide from '@/components/gamble-battle/GambleBattleGuide.vue';
import { gambleBattleWorkerType } from '@/features/gambleBattle/workerType';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  conditions,
  handKinds,
  matchedHands,
  expandedMatches,
  clearMatchedHands,
  sumThreshold,
  currentProgress,
  progressUpdatedAt,
  showGambleBattleGuide,
} from '@/features/gambleBattle/state';
import { useAppI18n } from '@/shared/i18n';
import { useTranslatedToast } from '@/shared/i18n/toast';
import { formatBackendError, normalizeBackendError } from '@/shared/errors';

const { t, n } = useAppI18n();
const toast = useTranslatedToast();
const { phase, isActive, canStart, canStop, tryStart, tryStop, outcome } =
  useAutomationTask('gambleBattle');
const submitting = ref(false);
const controlsPanel = ref<HTMLElement | null>(null);
watch(
  showGambleBattleGuide,
  async (visible: boolean): Promise<void> => {
    await nextTick();
    const target = visible
      ? controlsPanel.value?.querySelector<HTMLElement>('[role="dialog"]')
      : controlsPanel.value?.querySelector<HTMLElement>('[data-guide-trigger]');
    target?.focus();
  },
  { immediate: true },
);
const now = useNow({ interval: 1000 });
const elapsedSeconds = computed((): number =>
  Math.floor(
    currentProgress.value.elapsedSeconds +
      (isActive.value ? Math.max(0, now.value.getTime() - progressUpdatedAt.value) / 1000 : 0),
  ),
);
const hasCondition = computed(
  (): boolean => handKinds.some((hand): boolean => conditions[hand]) || conditions.sum !== null,
);
const thresholdValid = computed(
  (): boolean =>
    conditions.sum === null ||
    (Number.isInteger(sumThreshold.value) && sumThreshold.value >= 1 && sumThreshold.value <= 25),
);
const buttonLabel = computed((): string => {
  if (phase.value === 'stopping') return t('gambleBattle.stopping');
  if (phase.value === 'blocked') return t('gambleBattle.blocked');
  if (isActive.value) return t('gambleBattle.stop');
  return gambleBattleWorkerType.value === 'simulation'
    ? t('gambleBattle.startSimulation')
    : t('gambleBattle.start');
});
const errorMessage = computed((): string | null => {
  const result = outcome.value;
  if (result?.status !== 'failed') return null;
  const error = result.error;
  if (
    error.scope === 'worker' &&
    error.kind === 'capability' &&
    error.reason.kind === 'stoppedByUser'
  )
    return null;
  return formatBackendError(normalizeBackendError(error, { operation: 'automation' }));
});
function toggleSum(enabled: boolean): void {
  conditions.sum = enabled ? sumThreshold.value : null;
}

function changeThreshold(value: number | null): void {
  sumThreshold.value = value ?? 0;
  if (conditions.sum !== null) conditions.sum = sumThreshold.value;
}

async function toggleAutomation(): Promise<void> {
  submitting.value = true;
  try {
    if (canStart.value) {
      if (!hasCondition.value || !thresholdValid.value) return;
      await tryStart({ conditions: { ...conditions }, workerType: gambleBattleWorkerType.value });
    } else {
      await tryStop();
    }
  } catch (error: unknown) {
    const facts = normalizeBackendError(error, { operation: 'automation' });
    toast.add({ color: 'error' }, () => ({
      title: t('gambleBattle.failed'),
      description: formatBackendError(facts),
    }));
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div
    class="grid h-full min-h-0 grid-cols-1 gap-2 md:grid-cols-[minmax(260px,320px)_minmax(0,1fr)]"
  >
    <div ref="controlsPanel" class="relative min-h-0 min-w-0">
      <WorkspaceCard
        body-class="space-y-4"
        header-class="flex min-h-14 items-center [&>*]:w-full"
        :inert="showGambleBattleGuide"
      >
        <template #header>
          <div class="flex items-center justify-between gap-2">
            <h1 class="text-lg font-semibold text-highlighted" :title="t('gambleBattle.title')">
              {{ t('gambleBattle.heading') }}
            </h1>
            <div class="flex shrink-0 items-center gap-1">
              <UButton
                :aria-label="t('gambleBattle.guide.help')"
                color="neutral"
                data-guide-trigger
                icon="i-lucide-circle-help"
                size="sm"
                variant="outline"
                @click="showGambleBattleGuide = true"
              />
              <UButton
                :color="
                  isActive
                    ? 'error'
                    : gambleBattleWorkerType === 'simulation'
                      ? 'warning'
                      : 'primary'
                "
                :disabled="
                  submitting ||
                  (!canStart && !canStop) ||
                  (canStart && (!hasCondition || !thresholdValid))
                "
                :icon="isActive ? 'i-lucide-square' : 'i-lucide-play'"
                :label="buttonLabel"
                :loading="submitting"
                size="sm"
                @click="toggleAutomation"
              />
            </div>
          </div>
        </template>
        <UAlert
          v-if="errorMessage && !isActive"
          color="error"
          :description="errorMessage"
          icon="i-lucide-circle-alert"
          :title="t('gambleBattle.failed')"
          variant="subtle"
        />
        <dl aria-live="off" class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted">
          <div>
            <dt class="inline">{{ t('gambleBattle.elapsed') }}</dt>
            <dd class="inline tabular-nums">
              {{ t('gambleBattle.seconds', { count: elapsedSeconds }) }}
            </dd>
          </div>
          <div>
            <dt class="inline">{{ t('gambleBattle.round') }}</dt>
            <dd class="inline tabular-nums">{{ n(currentProgress.round) }}</dd>
          </div>
        </dl>
        <fieldset class="space-y-3" :disabled="isActive || submitting">
          <legend class="mb-2 flex items-center gap-1 text-sm font-medium text-highlighted">
            {{ t('gambleBattle.conditions') }}
          </legend>
          <div class="space-y-3">
            <USwitch
              v-for="hand in handKinds"
              :key="hand"
              v-model="conditions[hand]"
              class="w-full"
              :disabled="isActive || submitting"
              :label="t(`gambleBattle.hands.${hand}`)"
              size="sm"
            />
          </div>
          <div class="flex flex-wrap items-center gap-2 border-t border-default pt-3">
            <USwitch
              class="max-w-full"
              :disabled="isActive || submitting"
              :label="t('gambleBattle.sum')"
              :model-value="conditions.sum !== null"
              size="sm"
              @update:model-value="toggleSum"
            />
            <UInputNumber
              :aria-label="t('gambleBattle.threshold')"
              class="ml-auto w-28 shrink-0"
              :disabled="conditions.sum === null || isActive || submitting"
              :max="25"
              :min="1"
              :model-value="sumThreshold"
              size="sm"
              :step="1"
              @update:model-value="changeThreshold"
            />
            <p v-if="!thresholdValid" class="w-full text-xs text-error">
              {{ t('gambleBattle.invalidThreshold') }}
            </p>
          </div>
          <p v-if="!hasCondition" class="text-xs text-warning">
            {{ t('gambleBattle.selectCondition') }}
          </p>
        </fieldset>
      </WorkspaceCard>
      <GambleBattleGuide v-if="showGambleBattleGuide" @confirm="showGambleBattleGuide = false" />
    </div>

    <WorkspaceCard
      body-class="space-y-1.5 p-2!"
      header-class="flex min-h-14 items-center [&>*]:w-full"
    >
      <template #header>
        <div class="flex items-center justify-between gap-2">
          <h2 class="text-lg font-semibold text-highlighted">
            {{ t('gambleBattle.results') }}
            <span class="font-normal text-muted">{{ n(matchedHands.length) }}</span>
          </h2>
          <UButton
            color="neutral"
            :disabled="isActive || matchedHands.length === 0"
            icon="i-lucide-trash-2"
            :label="t('gambleBattle.clear')"
            size="xs"
            variant="outline"
            @click="clearMatchedHands"
          />
        </div>
      </template>
      <MatchedHandCard
        v-for="(match, matchIndex) in matchedHands"
        :key="matchIndex"
        v-model:expanded="expandedMatches[matchIndex]"
        :index="matchIndex"
        :match="match"
      />
      <div
        v-if="matchedHands.length === 0"
        class="flex h-full min-h-48 flex-col items-center justify-center gap-3 text-center text-muted"
      >
        <p class="text-sm">{{ t('gambleBattle.empty') }}</p>
      </div>
    </WorkspaceCard>
  </div>
</template>
