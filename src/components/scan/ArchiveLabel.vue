<script setup lang="ts">
defineProps<{ title: string; contentPreview?: string }>();
</script>

<template>
  <span
    class="block min-w-0 truncate text-sm font-medium"
    :title="contentPreview ? `${title} ${contentPreview}` : title"
  >
    <span>{{ title }}</span
    ><span v-if="contentPreview" class="ml-2 text-xs font-normal text-muted">{{
      contentPreview
    }}</span>
  </span>
</template>
