<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
const { t } = useAppI18n();
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center gap-2 font-bold text-highlighted">
      <UIcon class="size-5 text-primary" name="i-lucide-keyboard" />
      {{ t('imageViewer.shortcuts.title') }}
    </div>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">
        {{ t('imageViewer.shortcuts.view.title') }}
      </p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.view.pan') }}</span>
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
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.view.panImage') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd>
            <UIcon class="me-0.5" name="i-lucide-mouse" />
            {{ t('imageViewer.shortcuts.view.drag') }}
          </UKbd>
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">
        {{ t('imageViewer.shortcuts.rotation.title') }}
      </p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.rotateClockwise') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="R" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{
          t('imageViewer.shortcuts.rotation.counterClockwise')
        }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="shift" /> + <UKbd value="R" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.rotation.smooth') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="Q" />
          <UKbd value="E" />
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">
        {{ t('imageViewer.shortcuts.zoom.title') }}
      </p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.zoom.image') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="-" />
          <UKbd value="=" />
          <UKbd>
            <UIcon class="me-0.5" name="i-lucide-mouse" />
            {{ t('imageViewer.shortcuts.zoom.wheel') }}
          </UKbd>
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.zoom.smooth') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="[" />
          <UKbd value="]" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.zoom.preset') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd v-for="x in 9" :key="x" :value="x.toString()" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.resetView') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="0" />
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">
        {{ t('imageViewer.shortcuts.advanced.title') }}
      </p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.advanced.accelerate') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="shift" /> + <UKbd :value="t('imageViewer.shortcuts.advanced.actionKey')" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.advanced.download') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="meta" /> + <UKbd value="S" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">{{ t('imageViewer.shortcuts.advanced.openTab') }}</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="O" />
        </div>
      </div>
    </section>

    <USeparator />

    <p class="text-center text-xs text-warning">{{ t('imageViewer.shortcuts.inputMode') }}</p>
  </div>
</template>
