<script setup lang="ts">
import type { CSSProperties } from 'vue';
import { useColorMode } from '@vueuse/core';

defineProps<{
  modelValue: string;
  items: { id: string; lightLabel: string; darkLabel: string; chipStyle: CSSProperties }[];
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();
const colorMode = useColorMode();
</script>

<template>
  <div class="flex max-w-full shrink-0 flex-wrap gap-2 sm:w-96 sm:justify-end">
    <UTooltip
      v-for="color in items"
      :key="color.id"
      :text="colorMode === 'dark' ? color.darkLabel : color.lightLabel"
    >
      <UButton
        :aria-label="colorMode === 'dark' ? color.darkLabel : color.lightLabel"
        :aria-pressed="modelValue === color.id"
        class="size-8 justify-center p-1"
        color="neutral"
        variant="outline"
        @click="emit('update:modelValue', color.id)"
      >
        <span
          class="flex size-5 items-center justify-center rounded-full bg-(--color-light) text-white dark:bg-(--color-dark)"
          :style="color.chipStyle"
        >
          <UIcon
            v-if="modelValue === color.id"
            class="size-3"
            :class="color.id === 'grayscale' ? 'dark:text-black' : ''"
            name="i-lucide-check"
          />
        </span>
      </UButton>
    </UTooltip>
  </div>
</template>
