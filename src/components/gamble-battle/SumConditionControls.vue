<script setup lang="ts">
import { computed, useId, watch } from 'vue';
import { useAppI18n } from '@/shared/i18n';
import type { Comparison } from '@/shared/types/gambleBattle';
import { comparisonSymbols, sumThresholdRange } from '@/features/gambleBattle/comparison';

const props = defineProps<{ disabled: boolean; hasOtherConditions: boolean; valid: boolean }>();
const enabled = defineModel<boolean>('enabled', { required: true });
const threshold = defineModel<number>('threshold', { required: true });
const comparison = defineModel<Comparison>('comparison', { required: true });
const join = defineModel<'and' | 'or'>('join', { required: true });
const { t } = useAppI18n();
const switchId = useId();
const comparisons = (Object.keys(comparisonSymbols) as Comparison[]).map(
  (value: Comparison): { value: Comparison; label: string } => ({
    value,
    label: comparisonSymbols[value],
  }),
);
const joins = computed((): { value: 'and' | 'or'; label: string }[] => [
  { value: 'and', label: t('gambleBattle.conditions.join.and') },
  { value: 'or', label: t('gambleBattle.conditions.join.or') },
]);

const range = computed((): { min: number; max: number } => sumThresholdRange(comparison.value));
watch(comparison, (value: Comparison): void => {
  const { min, max } = sumThresholdRange(value);
  threshold.value = Math.min(max, Math.max(min, threshold.value));
});

// Measure the translated select instead of fixing its width. The wrapper owns both
// opacity and occupied space, so adjacent controls move with the fade.
function animateJoin(element: Element, done: () => void, entering: boolean): void {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
    done();
    return;
  }
  const width = `${element.scrollWidth}px`;
  const hidden = { opacity: 0, width: '0px' };
  const visible = { opacity: 1, width };
  const animation = element.animate(entering ? [hidden, visible] : [visible, hidden], {
    duration: 200,
    easing: 'ease',
  });
  animation.onfinish = done;
}
function cancel(element: Element): void {
  element.getAnimations().forEach((animation: Animation) => animation.cancel());
}
</script>

<template>
  <div class="rounded-lg border border-default p-2">
    <div class="flex flex-wrap items-center gap-2">
      <USwitch
        :id="switchId"
        v-model="enabled"
        :aria-label="t('gambleBattle.conditions.sum.label')"
        class="shrink-0"
        :disabled="props.disabled"
        size="sm"
      />
      <Transition
        :css="false"
        @enter="(element: Element, done: () => void) => animateJoin(element, done, true)"
        @enter-cancelled="cancel"
        @leave="(element: Element, done: () => void) => animateJoin(element, done, false)"
        @leave-cancelled="cancel"
      >
        <span v-if="props.hasOtherConditions" class="-me-2 inline-flex shrink-0 overflow-hidden">
          <span class="flex w-max pe-2">
            <USelect
              v-model="join"
              :aria-label="t('gambleBattle.conditions.join.label')"
              :disabled="props.disabled || !enabled"
              :items="joins"
              size="sm"
            />
          </span>
        </span>
      </Transition>
      <label class="shrink-0 text-sm text-highlighted" :for="switchId">
        {{ t('gambleBattle.conditions.sum.label') }}
      </label>
      <div class="flex min-w-0 items-center gap-2">
        <USelect
          v-model="comparison"
          :aria-label="t('gambleBattle.conditions.sum.comparison')"
          :disabled="props.disabled || !enabled"
          :items="comparisons"
          size="sm"
        />
        <UInputNumber
          v-model="threshold"
          :aria-label="t('gambleBattle.conditions.sum.threshold')"
          :decrement="false"
          :disabled="props.disabled || !enabled"
          :increment="false"
          :max="range.max"
          :min="range.min"
          size="sm"
          :ui="{ base: '[field-sizing:content] w-auto min-w-[3ch] text-center' }"
        />
      </div>
    </div>
    <p v-if="!props.valid" class="mt-2 text-sm text-error">
      {{ t('gambleBattle.conditions.sum.invalid', { min: range.min, max: range.max }) }}
    </p>
  </div>
</template>
