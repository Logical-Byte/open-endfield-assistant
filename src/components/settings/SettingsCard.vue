<script setup lang="ts">
const { icon = undefined, title = undefined } = defineProps<{
  icon?: string;
  title?: string;
}>();
</script>

<template>
  <UCard class="rounded-xl">
    <template #header>
      <slot name="header">
        <div class="flex items-center gap-2">
          <slot name="icon">
            <UIcon v-if="icon !== undefined" :name="icon" />
          </slot>
          <slot name="title">
            <span v-if="title !== undefined" class="font-semibold text-highlighted">
              {{ title }}
            </span>
          </slot>
        </div>
      </slot>
    </template>
    <slot name="body">
      <div class="divide-y divide-default *:py-4 *:first:pt-0 *:last:pb-0">
        <slot />
      </div>
    </slot>
  </UCard>
</template>
