<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from 'vue';

import DeveloperSettingsBody from './DeveloperSettingsBody.vue';
import SettingsCard from './SettingsCard.vue';

const enabled = ref<boolean>(false);
const rootElement = ref<HTMLElement>();
const anchorSpacerHeight = ref<number>(0);
let compensatedScrollContainer: HTMLElement | undefined;
let compensatedScrollTop = 0;
let initialSpacerHeight = 0;

function stopScrollCompensation(): void {
  compensatedScrollContainer?.removeEventListener('scroll', reclaimAnchorSpacer);
  compensatedScrollContainer = undefined;
}

function reclaimAnchorSpacer(): void {
  if (compensatedScrollContainer === undefined) {
    return;
  }
  const upwardDistance = compensatedScrollTop - compensatedScrollContainer.scrollTop;
  if (upwardDistance <= 0) {
    return;
  }
  anchorSpacerHeight.value = Math.max(0, initialSpacerHeight - upwardDistance);
  if (anchorSpacerHeight.value === 0) {
    stopScrollCompensation();
  }
}

async function toggleKeepingScrollAnchor(): Promise<void> {
  if (!enabled.value) {
    stopScrollCompensation();
    anchorSpacerHeight.value = 0;
    enabled.value = true;
    return;
  }

  const scrollContainer = document.querySelector<HTMLElement>('main');
  const toggleButton = rootElement.value?.querySelector<HTMLElement>('[data-developer-toggle]');

  if (scrollContainer === null || toggleButton === null || toggleButton === undefined) {
    stopScrollCompensation();
    anchorSpacerHeight.value = 0;
    enabled.value = false;
    return;
  }

  const buttonTopBefore = toggleButton.getBoundingClientRect().top;
  enabled.value = false;
  await nextTick();

  const buttonTopAfter = toggleButton.getBoundingClientRect().top;
  const clippedDistance = Math.max(0, buttonTopAfter - buttonTopBefore);
  if (clippedDistance === 0) {
    return;
  }

  anchorSpacerHeight.value = clippedDistance;
  await nextTick();
  scrollContainer.scrollTop += clippedDistance;
  compensatedScrollContainer = scrollContainer;
  compensatedScrollTop = scrollContainer.scrollTop;
  initialSpacerHeight = clippedDistance;
  scrollContainer.addEventListener('scroll', reclaimAnchorSpacer, { passive: true });
}

onBeforeUnmount(stopScrollCompensation);
</script>

<template>
  <div ref="rootElement">
    <SettingsCard id="developer" class="scroll-mt-8">
      <template #header>
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-1 items-center gap-3">
            <UIcon class="text-2xl text-primary" name="i-lucide-code-2" />
            <div class="flex-1">
              <p class="font-medium">开发者选项</p>
              <p class="text-sm text-dimmed">显示仅用于开发和故障排查的高级工具</p>
            </div>
          </div>
          <UButton
            color="neutral"
            data-developer-toggle
            :icon="enabled ? 'i-lucide-eye-off' : 'i-lucide-lock-keyhole-open'"
            :label="enabled ? '关闭开发者选项' : '开启开发者选项'"
            size="sm"
            variant="soft"
            @click="toggleKeepingScrollAnchor"
          />
        </div>
      </template>
      <DeveloperSettingsBody v-if="enabled" />
    </SettingsCard>
    <div aria-hidden="true" :style="{ height: `${anchorSpacerHeight}px` }" />
  </div>
</template>
