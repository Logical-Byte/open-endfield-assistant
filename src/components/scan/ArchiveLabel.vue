<script setup lang="ts">
defineProps<{
  title: string;
  contentPreview?: string;
  categoryLabel?: string;
  layout?: 'inline' | 'stacked';
  variant?: 'soft' | 'plain';
}>();
</script>

<template>
  <span
    class="block min-w-0 space-y-1 text-sm font-medium"
    :class="variant !== 'plain' && 'rounded-md bg-elevated p-2'"
    :title="contentPreview ? `${title} ${contentPreview}` : title"
  >
    <span class="block" :class="layout === 'stacked' ? 'space-y-1 break-words' : 'truncate'">
      <span :class="layout === 'stacked' && 'block'">{{ title }}</span
      ><span
        v-if="contentPreview"
        class="text-xs font-normal text-muted"
        :class="layout === 'stacked' ? 'block truncate' : 'ml-2'"
        >{{ contentPreview }}</span
      >
    </span>
    <span v-if="categoryLabel" class="flex items-start gap-1.5 text-xs font-normal text-muted">
      <UIcon class="mt-0.5 size-3.5 shrink-0" name="i-lucide-folder" />
      <span class="break-words">{{ categoryLabel }}</span>
    </span>
  </span>
</template>
