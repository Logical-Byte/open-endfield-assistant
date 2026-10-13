<script setup lang="ts">
import type { SettingsControl } from './settingsItem';
import { useSettingsControl } from './useSettingsControl';

const props = defineProps<{
  control: SettingsControl;
  ariaLabel?: string;
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
} = useSettingsControl(() => props.control);
</script>

<template>
  <USwitch
    v-if="control.kind === 'switch'"
    :aria-label="ariaLabel"
    :disabled="disabled"
    :model-value="value as boolean"
    @update:model-value="updateSwitch"
  />
  <div v-else-if="control.kind === 'slider'" class="flex w-56 items-center gap-2">
    <div class="flex-1">
      <USlider
        :aria-label="ariaLabel"
        :disabled="disabled"
        :max="control.max"
        :min="control.min"
        :model-value="value as number"
        :step="control.step"
        :tooltip="control.tooltip"
        @update:model-value="updateSlider"
      />
      <div v-if="control.marks" class="mt-1 flex justify-between text-xs text-dimmed tabular-nums">
        <span v-for="mark in control.marks" :key="mark">{{ control.formatValue(mark) }}</span>
      </div>
    </div>
    <span class="text-end text-sm tabular-nums" :class="control.marks ? 'w-12 shrink-0' : 'w-10'">{{
      control.formatValue(value as number)
    }}</span>
  </div>
  <USelect
    v-else-if="control.kind === 'select'"
    :aria-label="ariaLabel"
    class="w-56"
    :disabled="disabled"
    :items="items"
    :model-value="value as string"
    @update:model-value="updateChoice"
  />
  <UInput
    v-else-if="control.kind === 'input'"
    v-model.lazy="inputValue"
    :aria-label="ariaLabel"
    class="w-56"
    :disabled="disabled"
    :placeholder="placeholder"
    :type="control.type"
    @keydown.enter="blurOnEnter"
  />
  <div v-else-if="control.kind === 'choice-buttons'" class="flex shrink-0 flex-wrap gap-2">
    <UButton
      v-for="item in items"
      :key="item.value"
      :disabled="disabled"
      :label="item.label"
      :variant="value === item.value ? 'solid' : 'outline'"
      @click="updateChoice(item.value)"
    />
  </div>
  <UButton
    v-else
    class="shrink-0"
    :color="control.color"
    :disabled="disabled || buttonDisabled"
    :icon="control.icon"
    :label="buttonLabel"
    :loading="buttonLoading"
    :variant="control.variant"
    @click="control.onClick"
  />
</template>
