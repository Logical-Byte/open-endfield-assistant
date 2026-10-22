<script setup lang="ts">
import { onUnmounted, ref } from 'vue';
import { useAppI18n } from '@/shared/i18n';

defineProps<{ label: string; url: string }>();
const { t } = useAppI18n();
const active = ref(false);
let hovered = false;
let keyboardFocused = false;
let timer: ReturnType<typeof setTimeout> | undefined;

// 只有等待结束后才切换视觉状态，取消等待不会触发任何样式变化。
function schedule(): void {
  clearTimeout(timer);
  const next = hovered || keyboardFocused;
  if (next === active.value) return;
  timer = setTimeout(() => {
    active.value = next;
  }, 120);
}
function enter(): void {
  hovered = true;
  schedule();
}
function leave(): void {
  hovered = false;
  schedule();
}
function focus(event: FocusEvent): void {
  keyboardFocused = (event.currentTarget as HTMLElement).matches(':focus-visible');
  schedule();
}
function blur(): void {
  keyboardFocused = false;
  schedule();
}
onUnmounted(() => clearTimeout(timer));
</script>

<template>
  <UButton
    :aria-label="`${label} ${t('scan.goToOem')}`"
    class="archive-acquisition-link relative w-full justify-center overflow-hidden group-data-[compact=true]/archive:h-6 group-data-[compact=true]/archive:py-0 hover:bg-default"
    color="neutral"
    size="xs"
    target="_blank"
    :to="url"
    variant="outline"
    @blur="blur"
    @focus="focus"
    @mouseenter="enter"
    @mouseleave="leave"
  >
    <span
      aria-hidden="true"
      class="pointer-events-none inline-flex items-center justify-center text-sm font-normal transition-opacity duration-120 motion-reduce:transition-none"
      :class="active ? 'opacity-0' : 'opacity-100'"
    >
      {{ label }}
    </span>
    <!-- 独立叠层用于交叉淡化文字与背景，避免组件自带 hover 样式提前变化。 -->
    <span
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 flex items-center justify-center bg-inverted text-sm font-normal text-inverted transition-opacity duration-120 motion-reduce:transition-none"
      :class="active ? 'opacity-100' : 'opacity-0'"
      >{{ t('scan.goToOem') }}</span
    >
  </UButton>
</template>
