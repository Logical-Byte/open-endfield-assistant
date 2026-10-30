<script setup lang="ts">
import { computed, ref, type DeepReadonly } from 'vue';
import { TranslationComponent } from 'i18next-vue';
import { useAppI18n } from '@/shared/i18n';
import { openImagePreview } from '@/composables/image-preview';
import type { MatchedHand, StopReason } from '@/shared/types/gambleBattle';

const props = defineProps<{ match: DeepReadonly<MatchedHand>; index: number }>();
const expanded = defineModel<boolean>('expanded', { default: false });
const screenshotExpanded = ref(false);
const { t, n } = useAppI18n();
const summary = computed((): string => {
  const firstReason = props.match.reasons[0];
  return firstReason ? reasonLabel(firstReason) : '';
});
const total = computed((): number =>
  props.match.cards.reduce((sum, card): number => sum + card.rank, 0),
);
const handMatched = computed((): boolean =>
  props.match.reasons.some((reason): boolean => reason.kind === 'hand'),
);
const sumReason = computed(() => props.match.reasons.find((reason) => reason.kind === 'sum'));
const scoreReason = computed(() => props.match.reasons.find((reason) => reason.kind === 'score'));
const patternReason = computed(() =>
  props.match.reasons.find((reason) => reason.kind === 'pattern'),
);

function reasonLabel(reason: DeepReadonly<StopReason>): string {
  switch (reason.kind) {
    case 'hand':
      return t(`gambleBattle.hands.${reason.hand}`);
    case 'sum':
      return t('gambleBattle.sumMatched', {
        threshold: n(reason.threshold),
        total: n(reason.total),
      });
    case 'score':
      return t('gambleBattle.scoreMatched', {
        threshold: n(reason.threshold),
        score: n(reason.score),
      });
    case 'pattern':
      return t('gambleBattle.patternMatched', { pattern: reason.pattern });
  }
}

function previewMatch(): void {
  openImagePreview({
    url: props.match.image,
    name: t('gambleBattle.screenshot'),
    downloadName: 'Arena of Games.png',
  });
}
</script>

<template>
  <UCard as="article" :ui="{ body: 'p-0 sm:p-0' }">
    <UCollapsible
      v-model:open="expanded"
      :ui="{ content: 'overflow-hidden motion-reduce:animate-none' }"
      :unmount-on-hide="false"
    >
      <UButton
        :aria-expanded="expanded"
        class="min-h-10 w-full min-w-0 justify-start rounded-none px-3 py-2 text-left text-sm ring-0"
        color="neutral"
        icon="i-lucide-circle-check"
        :ui="{ leadingIcon: 'size-4 text-success' }"
        variant="ghost"
      >
        <span class="shrink-0 text-xs text-muted">#{{ n(index + 1) }}</span>
        <span class="flex min-w-0 flex-1 items-center gap-2">
          <span class="truncate" :title="summary">{{ summary }}</span>
          <UBadge
            v-if="match.reasons.length > 1"
            class="shrink-0"
            color="success"
            size="sm"
            variant="subtle"
          >
            +{{ n(match.reasons.length - 1) }}
          </UBadge>
        </span>
        <UIcon
          class="size-4 shrink-0 transition-transform duration-200 motion-reduce:transition-none"
          :class="expanded && 'rotate-180'"
          name="i-lucide-chevron-down"
        />
      </UButton>
      <template #content>
        <div class="space-y-4 border-t border-default p-4 text-sm text-toned">
          <section class="space-y-3">
            <div class="flex flex-wrap justify-center gap-2">
              <UBadge
                v-for="(card, cardIndex) in match.cards"
                :key="cardIndex"
                class="text-sm font-normal text-toned"
                color="neutral"
                size="md"
                variant="subtle"
              >
                {{
                  t('gambleBattle.dataplete', {
                    suit: t(`gambleBattle.suits.${card.suit}`),
                    rank: n(card.rank),
                  })
                }}
              </UBadge>
            </div>
            <p>
              <TranslationComponent :translation="t('gambleBattle.handTotal')">
                <template #value>
                  <UBadge
                    v-if="handMatched"
                    class="text-sm font-normal"
                    color="success"
                    size="md"
                    variant="soft"
                  >
                    {{
                      match.hand ? t(`gambleBattle.hands.${match.hand}`) : t('gambleBattle.noHand')
                    }}
                  </UBadge>
                  <span v-else>{{
                    match.hand ? t(`gambleBattle.hands.${match.hand}`) : t('gambleBattle.noHand')
                  }}</span>
                </template>
              </TranslationComponent>
              <UBadge
                v-if="match.lucky"
                class="ml-2 text-sm font-normal"
                color="primary"
                size="md"
                variant="subtle"
              >
                {{ t('gambleBattle.luckyBadge') }}
              </UBadge>
            </p>
            <p>
              <TranslationComponent
                :translation="
                  sumReason
                    ? t('gambleBattle.conditionSum', { threshold: n(sumReason.threshold) })
                    : t('gambleBattle.total', { total: '{value}' })
                "
              >
                <template #value>
                  <UBadge
                    v-if="sumReason"
                    class="text-sm font-normal"
                    color="success"
                    size="md"
                    variant="soft"
                    >{{ n(total) }}</UBadge
                  >
                  <span v-else>{{ n(total) }}</span>
                </template>
              </TranslationComponent>
            </p>
            <p>
              <TranslationComponent
                :translation="
                  scoreReason
                    ? t('gambleBattle.conditionScore', { threshold: n(scoreReason.threshold) })
                    : t('gambleBattle.scoreTotal', { score: '{value}' })
                "
              >
                <template #value>
                  <UBadge
                    v-if="scoreReason"
                    class="text-sm font-normal"
                    color="success"
                    size="md"
                    variant="soft"
                    >{{ n(match.score) }}</UBadge
                  >
                  <span v-else>{{ n(match.score) }}</span>
                </template>
              </TranslationComponent>
            </p>
            <p v-if="patternReason">
              <TranslationComponent :translation="t('gambleBattle.conditionPattern')">
                <template #value>
                  <UBadge class="text-sm font-normal" color="success" size="md" variant="soft">{{
                    patternReason.pattern
                  }}</UBadge>
                </template>
              </TranslationComponent>
            </p>
            <div class="flex flex-wrap gap-x-4 gap-y-2">
              <p>{{ t('gambleBattle.matchedRound', { round: n(match.round) }) }}</p>
              <p>
                {{
                  t('gambleBattle.matchedElapsed', {
                    seconds: n(Math.floor(match.elapsedSeconds)),
                  })
                }}
              </p>
            </div>
          </section>
          <USeparator />
          <UCollapsible
            v-model:open="screenshotExpanded"
            class="space-y-2"
            :ui="{ content: 'overflow-hidden motion-reduce:animate-none' }"
          >
            <UButton
              :aria-expanded="screenshotExpanded"
              color="neutral"
              :label="t('gambleBattle.originalScreenshot')"
              size="sm"
              :trailing-icon="screenshotExpanded ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
              variant="outline"
            />
            <template #content>
              <UButton
                :aria-label="t('gambleBattle.screenshot')"
                class="block w-full overflow-hidden p-0"
                color="neutral"
                variant="outline"
                @click="previewMatch"
              >
                <img :alt="t('gambleBattle.screenshot')" class="w-full" :src="match.image" />
              </UButton>
            </template>
          </UCollapsible>
        </div>
      </template>
    </UCollapsible>
  </UCard>
</template>
