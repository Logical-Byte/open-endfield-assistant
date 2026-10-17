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
        class="size-8 justify-center bg-(--color-light) p-1 hover:bg-(--color-light) active:bg-(--color-light) dark:bg-(--color-dark) dark:hover:bg-(--color-dark) dark:active:bg-(--color-dark)"
        :class="modelValue === color.id ? 'outline-highlighted outline-2 outline-offset-2' : ''"
        color="neutral"
        :style="color.chipStyle"
        variant="outline"
        @click="emit('update:modelValue', color.id)"
      />
    </UTooltip>
  </div>
</template>
