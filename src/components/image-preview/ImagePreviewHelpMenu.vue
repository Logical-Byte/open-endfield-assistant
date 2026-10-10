<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
const { t } = useAppI18n();
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center gap-2 font-bold text-highlighted">
      <UIcon class="size-5 text-primary" name="i-lucide-keyboard" />
      {{ t('image.guide.title') }}
    </div>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">{{ t('image.guide.view') }}</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.panView') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="W" />
          <UKbd value="A" />
          <UKbd value="S" />
          <UKbd value="D" />
          <UKbd value="arrowup" />
          <UKbd value="arrowleft" />
          <UKbd value="arrowdown" />
          <UKbd value="arrowright" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.panImage') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd>
            <UIcon class="me-0.5" name="i-lucide-mouse" />
            {{ t('image.guide.drag') }}
          </UKbd>
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">{{ t('image.guide.rotation') }}</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.rotateClockwise') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="R" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.rotateCounterclockwise') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="shift" /> + <UKbd value="R" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.smoothRotation') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="Q" />
          <UKbd value="E" />
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">{{ t('image.guide.zoom') }}</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.zoomImage') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="-" />
          <UKbd value="=" />
          <UKbd>
            <UIcon class="me-0.5" name="i-lucide-mouse" />
            {{ t('image.guide.wheel') }}
          </UKbd>
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.smoothZoom') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="[" />
          <UKbd value="]" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.preset') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd v-for="x in 9" :key="x" :value="x.toString()" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.reset') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="0" />
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">{{ t('image.guide.advanced') }}</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.accelerate') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="shift" /> + <UKbd :value="t('image.guide.actionKey')" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.download') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="meta" /> + <UKbd value="S" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('image.guide.openTab') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="O" />
        </div>
      </div>
    </section>

    <USeparator />

    <p class="text-center text-xs text-warning">{{ t('image.guide.inputMode') }}</p>
  </div>
</template>
