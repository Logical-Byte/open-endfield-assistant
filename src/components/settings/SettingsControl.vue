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
      <div v-if="item.marks" class="mt-1 flex justify-between text-xs text-dimmed tabular-nums">
        <span v-for="mark in item.marks" :key="mark">{{ item.formatValue(mark) }}</span>
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
        <UBadge
          v-bind="choice.badge"
          class="pointer-events-none absolute -top-2 -right-2 rounded-full"
          size="xs"
          variant="soft"
        />
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
