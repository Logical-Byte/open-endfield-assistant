<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { TranslationComponent } from 'i18next-vue';
import { onMounted, ref } from 'vue';

const { t } = useAppI18n();
const emit = defineEmits<{ confirm: [] }>();

const guideRoot = ref<HTMLElement | null>(null);

onMounted((): void => {
  guideRoot.value?.focus();
});
</script>

<template>
  <section
    ref="guideRoot"
    :aria-label="t('gambleBattle.guide.title')"
    class="flex h-full flex-col items-center justify-center gap-10 px-6 text-center outline-none"
    tabindex="-1"
  >
    <ol class="inline-flex flex-col gap-5 text-left text-2xl leading-relaxed font-semibold">
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">1.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.open')"
            ><template #resolution><strong class="text-primary">1280 × 720</strong></template
            ><template #windowed
              ><strong class="text-primary">{{ t('help.windowed') }}</strong></template
            ><template #language
              ><strong class="text-primary">{{
                t('settings.language.game.fixed')
              }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">2.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.hdr')"
            ><template #hdr
              ><strong class="text-primary">{{ t('help.hdr.disable') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">3.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.arena')"
            ><template #screen
              ><strong class="text-primary">{{ t('help.gambleBattle.screen') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">4.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.target')"
            ><template #targetHands
              ><strong class="text-primary">{{
                t('help.gambleBattle.targetHands')
              }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">5.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.start')"
            ><template #start
              ><strong class="text-primary">{{ t('gambleBattle.start') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
    </ol>

    <UButton :label="t('gambleBattle.guide.confirm')" size="sm" @click="emit('confirm')" />
  </section>
</template>
