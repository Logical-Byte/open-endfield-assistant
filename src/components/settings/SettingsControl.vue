<script setup lang="ts">
import type { SettingsItemDefinition } from './settingsItem';
import { useSettingsControl } from './useSettingsControl';

const props = defineProps<{
  /** 控件连接和选项。整项展示状态由外层 SettingsItem 处理。 */
  item: SettingsItemDefinition;
  /** 自定义插槽内使用时，显式传入对应设置项的标题。 */
  ariaLabel?: string;
  /** 接收 SettingsItem 或其插槽提供的整项禁用状态。 */
  disabled?: boolean;
}>();

const {
  value,
  inputValue,
  items,
  placeholder,
  buttonLabel,
  buttonDisabled,
  buttonLoading,
  updateSwitch,
  updateSlider,
  updateChoice,
  blurOnEnter,
} = useSettingsControl((): SettingsItemDefinition => props.item);

// 保持 lazy 修饰符的引用稳定，避免内联 item 更新时 UInput 重渲染并清掉未提交的文本。
const inputModelModifiers: { lazy: boolean } = { lazy: true };

// 刻度标签按数值在滑块轨道上的百分比定位，中心与对应刻度对齐。
function markOffset(min: number, max: number, mark: number): string {
  const ratio = max === min ? 0 : (mark - min) / (max - min);
  return `${Math.min(Math.max(ratio, 0), 1) * 100}%`;
}
</script>

<template>
  <USwitch
    v-if="item.kind === 'switch'"
    :aria-label="ariaLabel"
    :disabled="disabled"
    :model-value="value as boolean"
    @update:model-value="updateSwitch"
  />
  <div v-else-if="item.kind === 'slider'" class="flex w-56 items-center gap-2">
    <div class="flex-1">
      <USlider
        :aria-label="ariaLabel"
        :disabled="disabled"
        :max="item.max"
        :min="item.min"
        :model-value="value as number"
        :step="item.step"
        :tooltip="item.tooltip"
        @update:model-value="updateSlider"
      />
      <!-- mx-2 让定位容器与滑块轨道可用区间一致：滑块 thumb 为 size-4，居中含 0.5rem 内缩。 -->
      <div v-if="item.marks" class="relative mx-2 mt-1 h-4 text-xs text-dimmed tabular-nums">
        <span
          v-for="mark in item.marks"
          :key="mark"
          class="absolute top-1/2 -translate-x-1/2 -translate-y-1/2 whitespace-nowrap"
          :style="{ left: markOffset(item.min, item.max, mark) }"
          >{{ item.formatValue(mark) }}</span
        >
      </div>
    </div>
    <span class="text-end text-sm tabular-nums" :class="item.marks ? 'w-12 shrink-0' : 'w-10'">{{
      item.formatValue(value as number)
    }}</span>
  </div>
  <USelect
    v-else-if="item.kind === 'select'"
    :aria-label="ariaLabel"
    class="w-56"
    :disabled="disabled"
    :items="items"
    :model-value="value as string"
    @update:model-value="updateChoice"
  />
  <UInput
    v-else-if="item.kind === 'input'"
    v-model="inputValue"
    :aria-label="ariaLabel"
    class="w-56"
    :disabled="disabled"
    :model-modifiers="inputModelModifiers"
    :placeholder="placeholder"
    :type="item.type"
    @keydown.enter="blurOnEnter"
  />
  <div v-else-if="item.kind === 'choice-buttons'" class="flex shrink-0 flex-wrap gap-2">
    <UButton
      v-for="choice in items"
      :key="choice.value"
      class="relative"
      :disabled="disabled"
      :label="choice.label"
      :variant="value === choice.value ? 'solid' : 'outline'"
      @click="updateChoice(choice.value)"
    >
      <template v-if="choice.badge" #trailing>
        <!-- 不透明底层隔开按钮颜色，保留 subtle 自带的浅色底和描边。 -->
        <span
          class="pointer-events-none absolute -top-2 -right-2 inline-flex rounded-full bg-default"
        >
          <UBadge v-bind="choice.badge" class="rounded-full" size="xs" variant="subtle" />
        </span>
      </template>
    </UButton>
  </div>
  <UButton
    v-else
    class="shrink-0"
    :color="item.color"
    :disabled="disabled || buttonDisabled"
    :icon="item.icon"
    :label="buttonLabel"
    :loading="buttonLoading"
    :variant="item.variant"
    @click="item.onClick"
  />
</template>
