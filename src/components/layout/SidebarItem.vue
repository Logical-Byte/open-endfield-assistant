<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router';

defineOptions({ inheritAttrs: false });
withDefaults(
  defineProps<{
    icon: string;
    label?: string;
    tooltip?: string;
    iconOnly?: boolean;
    collapsed?: boolean;
    to?: RouteLocationRaw;
    active?: boolean;
    loading?: boolean;
  }>(),
  {
    label: '',
    tooltip: '',
    iconOnly: false,
    collapsed: false,
    to: undefined,
    active: undefined,
    loading: false,
  },
);
</script>

<template>
  <!-- 点击区域高 34px，背景内收后高 32px。图标按钮左右各留 3px，与 40px 图标列对齐。 -->
  <UTooltip
    :content="{ side: 'right' }"
    :disabled="!iconOnly && !!label && !collapsed"
    :text="tooltip || label"
  >
    <UButton
      v-bind="$attrs"
      :active="active"
      active-class="sidebar-item-active"
      active-color="primary"
      active-variant="solid"
      :aria-label="tooltip || label"
      class="sidebar-item h-8.5 cursor-pointer gap-0 p-0 text-start transition-colors"
      :class="iconOnly ? 'mx-0.75 flex w-8.5' : 'grid w-full grid-cols-[40px_minmax(0,1fr)]'"
      color="neutral"
      size="md"
      :to="to"
      :ui="{
        base: 'relative isolate bg-transparent before:pointer-events-none before:absolute before:inset-px before:-z-1 before:rounded-[inherit] before:transition-colors hover:bg-transparent hover:text-primary hover:before:bg-accented/50 focus-visible:text-primary focus-visible:before:bg-accented/50 active:bg-transparent',
      }"
      variant="ghost"
    >
      <template #leading>
        <span
          class="relative flex h-8.5 shrink-0 items-center justify-center"
          :class="iconOnly ? 'w-8.5' : 'w-10'"
        >
          <UIcon
            class="size-5 shrink-0"
            :class="loading && 'animate-spin'"
            :name="loading ? 'i-lucide-loader-circle' : icon"
          />
          <slot name="indicator" />
        </span>
      </template>
      <span v-if="!iconOnly" :class="collapsed ? 'sr-only' : 'min-w-0 truncate pe-1'">{{
        label
      }}</span>
    </UButton>
  </UTooltip>
</template>

<style scoped>
/* active-class 与按钮共用选中判断，支持路由和分类导航。高亮背景四边各内收 1px。 */
.sidebar-item-active::before {
  background-color: var(--ui-primary);
}

.sidebar-item-active:is(:hover, :active, :focus-visible) {
  color: var(--ui-text-inverted);
}
</style>
