<script setup lang="ts">
import { computed, type DeepReadonly } from 'vue';
import { useAppI18n } from '@/shared/i18n';
import { openImagePreview } from '@/composables/image-preview';
import type { MatchedHand, StopReason } from '@/shared/types/gambleBattle';

const props = defineProps<{ match: DeepReadonly<MatchedHand>; index: number }>();
const expanded = defineModel<boolean>('expanded', { default: false });
const { t, n } = useAppI18n();
const summary = computed((): string => props.match.reasons.map(reasonLabel).join(' / '));
const total = computed((): number =>
  props.match.cards.reduce((sum, card): number => sum + card.rank, 0),
);

function reasonLabel(reason: DeepReadonly<StopReason>): string {
  return reason.kind === 'hand'
    ? t(`gambleBattle.hands.${reason.hand}`)
    : t('gambleBattle.sumMatched', { threshold: n(reason.threshold), total: n(reason.total) });
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
        <span class="min-w-0 flex-1 truncate" :title="summary">{{ summary }}</span>
        <UIcon
          class="size-4 shrink-0 transition-transform duration-200 motion-reduce:transition-none"
          :class="expanded && 'rotate-180'"
          name="i-lucide-chevron-down"
        />
      </UButton>
      <template #content>
        <div class="space-y-3 border-t border-default p-3 text-sm">
          <p class="text-xs text-muted">
            {{
              t('gambleBattle.matchProgress', {
                round: n(match.round),
                seconds: n(Math.floor(match.elapsedSeconds)),
              })
            }}
          </p>
          <ul class="space-y-1 text-toned">
            <li v-for="(reason, reasonIndex) in match.reasons" :key="reasonIndex">
              {{ reasonLabel(reason) }}
            </li>
          </ul>
          <div class="flex flex-wrap gap-2">
            <UBadge
              v-for="(card, cardIndex) in match.cards"
              :key="cardIndex"
              color="neutral"
              size="lg"
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
          <p class="text-muted">{{ t('gambleBattle.total', { total: n(total) }) }}</p>
          <UButton
            :aria-label="t('gambleBattle.screenshot')"
            class="block w-full overflow-hidden p-0"
            color="neutral"
            variant="outline"
            @click="previewMatch"
          >
            <img :alt="t('gambleBattle.screenshot')" class="w-full" :src="match.image" />
          </UButton>
        </div>
      </template>
    </UCollapsible>
  </UCard>
</template>
