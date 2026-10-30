<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import WorkspaceCard from '@/components/layout/WorkspaceCard.vue';
import MatchedHandCard from '@/components/gamble-battle/MatchedHandCard.vue';
import GambleBattleGuide from '@/components/gamble-battle/GambleBattleGuide.vue';
import { gambleBattleWorkerType } from '@/features/gambleBattle/workerType';
import { useGambleBattleControls } from '@/features/gambleBattle/controls';
import {
  conditions,
  handKinds,
  matchedHands,
  expandedMatches,
  clearMatchedHands,
  sumThreshold,
  scoreThreshold,
  bpPattern,
  showGambleBattleGuide,
} from '@/features/gambleBattle/state';
import { useAppI18n } from '@/shared/i18n';
import { formatBackendError, normalizeBackendError } from '@/shared/errors';

const { t, n } = useAppI18n();
const { task, submitting, hasCondition, sumValid, scoreValid, patternValid, toggleAutomation } =
  useGambleBattleControls();
const { phase, isActive, canStart, canStop, outcome } = task;
// 教程由 v-if 替换整页内容。关闭教程后，把焦点交还给重新挂载的教程入口。
const controlsPanel = ref<HTMLElement | null>(null);
watch(showGambleBattleGuide, async (visible: boolean): Promise<void> => {
  if (visible) return;
  await nextTick();
  controlsPanel.value?.querySelector<HTMLElement>('[data-guide-trigger]')?.focus();
});
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

function toggleScore(enabled: boolean): void {
  conditions.score = enabled ? scoreThreshold.value : null;
}

function changeScore(value: number | null): void {
  scoreThreshold.value = value ?? 0;
  if (conditions.score !== null) conditions.score = scoreThreshold.value;
}

function togglePattern(enabled: boolean): void {
  conditions.pattern = enabled ? bpPattern.value : null;
}

function changePattern(value: string | number): void {
  bpPattern.value = String(value)
    .replace(/[^1-5]/g, '')
    .slice(0, 5);
  if (conditions.pattern !== null) conditions.pattern = bpPattern.value;
}
</script>

<template>
  <GambleBattleGuide v-if="showGambleBattleGuide" @confirm="showGambleBattleGuide = false" />
  <div
    v-else
    class="grid h-full min-h-0 grid-cols-1 gap-2 md:grid-cols-[minmax(260px,320px)_minmax(0,1fr)]"
  >
    <div ref="controlsPanel" class="min-h-0 min-w-0">
      <WorkspaceCard body-class="space-y-4" header-class="flex min-h-14 items-center [&>*]:w-full">
        <template #header>
          <div class="flex items-center justify-between gap-2">
            <div class="flex min-w-0 items-center gap-2">
              <h1
                class="truncate text-lg font-semibold text-highlighted"
                :title="t('gambleBattle.title')"
              >
                {{ t('gambleBattle.heading') }}
              </h1>
              <UBadge class="shrink-0 rounded-full" color="success" size="sm" variant="outline">
                {{ t('gambleBattle.event') }}
              </UBadge>
            </div>
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
                  (canStart && (!hasCondition || !sumValid || !scoreValid || !patternValid))
                "
                :icon="isActive ? 'i-lucide-square' : 'i-lucide-play'"
                :label="buttonLabel"
                :loading="submitting"
                size="sm"
                @click="toggleAutomation"
              >
                <template #trailing>
                  <UKbd
                    class="bg-transparent font-bold text-inherit ring-current/30"
                    size="sm"
                    value=";"
                  />
                </template>
              </UButton>
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
        <fieldset class="space-y-1" :disabled="isActive || submitting">
          <legend class="mb-2 flex items-center gap-1 text-sm font-medium text-highlighted">
            {{ t('gambleBattle.conditions') }}
          </legend>
          <USwitch
            v-for="hand in handKinds"
            :key="hand"
            v-model="conditions[hand]"
            class="min-h-7 w-full items-center"
            :disabled="isActive || submitting"
            :label="t(`gambleBattle.hands.${hand}`)"
            size="sm"
            :ui="{ wrapper: 'text-sm', container: 'h-5' }"
          />
          <div class="flex flex-wrap items-center gap-2">
            <USwitch
              class="min-h-7 max-w-full items-center"
              :disabled="isActive || submitting"
              :label="t('gambleBattle.sum')"
              :model-value="conditions.sum !== null"
              size="sm"
              :ui="{ wrapper: 'text-sm', container: 'h-5' }"
              @update:model-value="toggleSum"
            />
            <UInputNumber
              :aria-label="t('gambleBattle.threshold')"
              class="ml-auto w-28 shrink-0"
              :disabled="conditions.sum === null || isActive || submitting"
              :format-options="{ useGrouping: false }"
              :max="25"
              :min="1"
              :model-value="sumThreshold"
              size="sm"
              :step="1"
              @update:model-value="changeThreshold"
            />
            <p v-if="!sumValid" class="w-full text-sm text-error">
              {{ t('gambleBattle.invalidThreshold') }}
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <USwitch
              class="min-h-7 max-w-full items-center"
              :disabled="isActive || submitting"
              :label="t('gambleBattle.score')"
              :model-value="conditions.score !== null"
              size="sm"
              :ui="{ wrapper: 'text-sm', container: 'h-5' }"
              @update:model-value="toggleScore"
            />
            <UInputNumber
              :aria-label="t('gambleBattle.scoreThreshold')"
              class="ml-auto w-28 shrink-0"
              :disabled="conditions.score === null || isActive || submitting"
              :format-options="{ useGrouping: false }"
              :min="1"
              :model-value="scoreThreshold"
              size="sm"
              :step="1000"
              @update:model-value="changeScore"
            />
            <p v-if="!scoreValid" class="w-full text-sm text-error">
              {{ t('gambleBattle.invalidScore') }}
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <USwitch
              class="min-h-7 max-w-full items-center"
              :disabled="isActive || submitting"
              :label="t('gambleBattle.pattern')"
              :model-value="conditions.pattern !== null"
              size="sm"
              :ui="{ wrapper: 'text-sm', container: 'h-5' }"
              @update:model-value="togglePattern"
            />
            <UInput
              :aria-label="t('gambleBattle.patternInput')"
              class="ml-auto w-28 shrink-0"
              :disabled="conditions.pattern === null || isActive || submitting"
              maxlength="5"
              :model-value="bpPattern"
              size="sm"
              @update:model-value="changePattern"
            />
            <p v-if="!patternValid" class="w-full text-sm text-error">
              {{ t('gambleBattle.invalidPattern') }}
            </p>
          </div>
          <p v-if="!hasCondition" class="text-sm text-warning">
            {{ t('gambleBattle.selectCondition') }}
          </p>
        </fieldset>
      </WorkspaceCard>
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
