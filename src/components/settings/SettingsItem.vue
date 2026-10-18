<script setup lang="ts">
import { computed, toValue, type ComputedRef } from 'vue';
import SettingsControl from './SettingsControl.vue';
import {
  settingsSkeletonClasses,
  type SettingsItemDefinition,
  type SettingsItemState,
  type SettingsSkeleton,
} from './settingsItem';

const {
  icon = undefined,
  title = undefined,
  description = undefined,
  item = undefined,
  skeleton = undefined,
} = defineProps<{
  /** 设置项图标，使用完整 Lucide 图标名。 */
  icon?: string;
  /** 设置项标题，同时作为普通输入控件的 ARIA 标签。 */
  title?: string;
  /** 简单说明文案。复合说明可使用 description 插槽。 */
  description?: string;
  /** 统一的控件与展示状态描述，默认插槽通过 item 复用它。省略时支持纯插槽内容。 */
  item?: SettingsItemDefinition;
  /** 覆盖按 item.kind 推导的加载骨架形状。 */
  skeleton?: SettingsSkeleton;
}>();

// 在 computed 内读取描述中的 ref/getter，保持状态更新与语言切换响应式。
const state: ComputedRef<SettingsItemState> = computed(
  (): SettingsItemState => toValue(item?.state) ?? { status: 'ready' },
);
const skeletonClass: ComputedRef<string> = computed(
  (): string => settingsSkeletonClasses[skeleton ?? item?.kind ?? 'input'],
);
</script>

<template>
  <div
    class="flex flex-col items-stretch justify-between gap-4 @min-[640px]/settings:flex-row @min-[640px]/settings:items-center"
  >
    <div class="flex min-w-0 flex-1 items-center gap-3">
      <slot name="left">
        <slot name="icon">
          <UIcon v-if="icon !== undefined" class="text-2xl text-primary" :name="icon" />
        </slot>
        <div class="min-w-0 flex-1">
          <slot name="title">
            <p v-if="title !== undefined" class="font-medium">{{ title }}</p>
          </slot>
          <slot name="description">
            <p v-if="description !== undefined" class="text-sm text-dimmed">{{ description }}</p>
          </slot>
        </div>
      </slot>
    </div>
    <USkeleton v-if="state.status === 'loading'" :class="skeletonClass" />
    <UBadge
      v-else-if="state.status === 'unavailable'"
      color="neutral"
      :label="state.label"
      :variant="state.variant ?? 'soft'"
    />
    <slot v-else :disabled="state.status === 'disabled'" :item="item">
      <SettingsControl
        v-if="item"
        :aria-label="title"
        :disabled="state.status === 'disabled'"
        :item="item"
      />
    </slot>
  </div>
</template>
