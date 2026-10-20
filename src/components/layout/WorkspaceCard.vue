<script setup lang="ts">
const { bodyClass = 'p-3' } = defineProps<{ bodyClass?: string }>();
</script>

<template>
  <!-- 实际边框包含在盒子内，贴合工作区时也不会被滚动容器裁切。 -->
  <UCard
    class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border border-default ring-0"
    :ui="{
      header: 'shrink-0 p-3 sm:px-3',
      body: `min-h-0 flex-1 overflow-y-auto sm:p-3 ${bodyClass}`,
    }"
  >
    <template v-if="$slots.header" #header><slot name="header" /></template>
    <slot />
  </UCard>
</template>
