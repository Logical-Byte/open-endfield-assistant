<script setup lang="ts">
import type { CSSProperties } from 'vue';

const props = defineProps<{
  label: string;
  icon?: string;
  chipStyle?: CSSProperties;
  selected?: boolean;
}>();
</script>

<template>
  <UButton
    color="neutral"
    :icon="props.icon"
    :label="props.label"
    size="sm"
    :variant="props.selected ? 'subtle' : 'outline'"
  >
    <template v-if="props.chipStyle" #leading>
      <span
        class="inline-block size-2 rounded-full bg-(--color-light) dark:bg-(--color-dark)"
        :style="props.chipStyle"
      />
    </template>
  </UButton>
</template>
