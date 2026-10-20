<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router';

defineOptions({ inheritAttrs: false });
withDefaults(
  defineProps<{
    icon: string;
    label?: string;
    tooltip?: string;
    collapsed?: boolean;
    to?: RouteLocationRaw;
    active?: boolean;
    loading?: boolean;
  }>(),
  { label: '', tooltip: '', collapsed: false, to: undefined, active: undefined, loading: false },
);
</script>

<template>
  <!-- 所有条目共享图标列、行高和交互样式。空文字只影响文字区，不改变图标的位置。 -->
  <UTooltip :content="{ side: 'right' }" :disabled="!!label && !collapsed" :text="tooltip || label">
    <UButton
      v-bind="$attrs"
      :active="active"
      active-color="primary"
      active-variant="solid"
      :aria-label="tooltip || label"
      class="sidebar-item grid h-9 w-full cursor-pointer grid-cols-[40px_minmax(0,1fr)] gap-0 p-0 text-start transition-colors"
      :class="active && 'sidebar-item-active'"
      color="neutral"
      :to="to"
      variant="ghost"
    >
      <template #leading>
        <span class="relative flex h-9 w-10 shrink-0 items-center justify-center">
          <UIcon
            class="size-5 shrink-0"
            :class="loading && 'animate-spin'"
            :name="loading ? 'i-lucide-loader-circle' : icon"
          />
          <slot name="indicator" />
        </span>
      </template>
      <span :class="collapsed ? 'sr-only' : 'min-w-0 truncate pe-1'">{{ label }}</span>
    </UButton>
  </UTooltip>
</template>

<style scoped>
/* 悬停只表示选择意图，实色背景始终留给当前选中项。 */
.sidebar-item:not([aria-current='page']):not(.sidebar-item-active):is(:hover, :focus-visible) {
  background-color: transparent;
  color: var(--ui-primary);
}

.sidebar-item:is([aria-current='page'], .sidebar-item-active):is(:hover, :active) {
  background-color: var(--ui-primary);
  color: var(--ui-text-inverted);
}
</style>
