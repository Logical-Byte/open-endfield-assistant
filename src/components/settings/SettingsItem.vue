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
  <div class="flex flex-col items-stretch justify-between gap-4 sm:flex-row sm:items-center">
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
    <slot />
  </div>
</template>
