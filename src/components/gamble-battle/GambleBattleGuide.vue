<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';

const { t } = useAppI18n();
const emit = defineEmits<{ confirm: [] }>();
</script>

<template>
  <section
    :aria-label="t('gambleBattle.guide.title')"
    class="absolute inset-0 z-20 flex flex-col justify-center gap-6 overflow-y-auto rounded-xl border border-default bg-default p-6 text-sm outline-none"
    role="dialog"
    tabindex="-1"
  >
    <div class="space-y-3">
      <h2 class="text-lg font-semibold text-highlighted">{{ t('gambleBattle.guide.title') }}</h2>
      <p class="leading-relaxed text-toned">{{ t('gambleBattle.guide.description') }}</p>
    </div>
    <ol class="list-decimal space-y-3 pl-5 leading-relaxed text-toned">
      <li>{{ t('gambleBattle.guide.open') }}</li>
      <li>{{ t('gambleBattle.guide.start') }}</li>
    </ol>
    <UButton
      class="self-center"
      :label="t('gambleBattle.guide.confirm')"
      size="sm"
      @click="emit('confirm')"
    />
  </section>
</template>
