<script setup lang="ts">
defineProps<{
  title: string;
  contentPreview?: string;
  preview?: 'hidden' | 'same-line' | 'next-line';
  categoryLabel?: string;
  variant?: 'soft' | 'plain';
}>();
</script>

<template>
  <span
    class="block min-w-0 space-y-1 text-sm font-medium"
    :class="variant !== 'plain' && 'rounded-md bg-elevated p-2'"
    :title="contentPreview ? `${title} ${contentPreview}` : title"
  >
    <!-- 同行时先给标题所需空间，摘要至少保留约四个字符，之后标题才开始省略。 -->
    <span
      class="grid min-w-0"
      :class="
        contentPreview && preview !== 'hidden' && preview !== 'next-line'
          ? 'grid-cols-[minmax(0,max-content)_minmax(min(4em,40%),1fr)] items-baseline gap-x-2'
          : 'grid-cols-1 gap-y-1'
      "
    >
      <span class="min-w-0 truncate">{{ title }}</span>
      <span
        v-if="contentPreview && preview !== 'hidden'"
        class="min-w-0 truncate text-xs font-normal text-muted"
        >{{ contentPreview }}</span
      >
    </span>
    <span v-if="categoryLabel" class="flex items-start gap-1.5 text-xs font-normal text-muted">
      <UIcon class="mt-0.5 size-3.5 shrink-0" name="i-lucide-folder" />
      <span class="min-w-0 truncate">{{ categoryLabel }}</span>
    </span>
  </span>
</template>
