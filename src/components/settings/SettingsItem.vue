<script setup lang="ts">
const {
  icon = undefined,
  title = undefined,
  description = undefined,
} = defineProps<{
  icon?: string;
  title?: string;
  description?: string;
}>();
</script>

<template>
  <div class="flex items-center justify-between gap-4">
    <div class="flex flex-1 items-center gap-3">
      <slot name="left">
        <slot name="icon">
          <UIcon v-if="icon !== undefined" class="text-2xl text-primary" :name="icon" />
        </slot>
        <div class="flex-1">
          <slot name="title">
            <p v-if="title !== undefined" class="font-medium">{{ title }}</p>
          </slot>
          <slot name="description">
            <p v-if="description !== undefined" class="text-sm text-dimmed">{{ description }}</p>
          </slot>
        </div>
      </slot>
    </div>
    <slot />
  </div>
</template>
