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
          ><TranslationComponent :translation="t('gambleBattle.guide.setup')"
            ><template #language
              ><strong class="text-primary">{{
                t('settings.language.game.name')
              }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">2.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.disableHdr')"
            ><template #hdr
              ><strong class="text-primary">{{ t('common.disableHdr') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">3.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.openArena')"
            ><template #screen
              ><strong class="text-primary">{{ t('help.arena.screen') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">4.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.selectTarget')"
            ><template #targetHands
              ><strong class="text-primary">{{ t('help.arena.targetHands') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">5.</span>
        <span
          ><TranslationComponent :translation="t('gambleBattle.guide.startDrawing')"
            ><template #start
              ><strong class="text-primary">{{ t('gambleBattle.start') }}</strong></template
            ></TranslationComponent
          ></span
        >
      </li>
    </ol>

    <UButton :label="t('common.gotIt')" size="sm" @click="emit('confirm')" />
  </section>
</template>
