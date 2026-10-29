<script setup lang="ts">
import { ref, watch } from 'vue';

const props = defineProps<{
  title: string;
  contentPreview?: string;
  preview?: 'hidden' | 'same-line' | 'next-line';
  // 摘要淡出后释放布局空间，让标题使用整行。
  previewHidden?: boolean;
  categoryLabel?: string;
  variant?: 'soft' | 'plain';
}>();
const previewLeaving = ref(false);
watch(
  () => props.previewHidden,
  (hidden): void => {
    previewLeaving.value = !!hidden;
  },
  { flush: 'sync' },
);
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
        contentPreview &&
        preview !== 'hidden' &&
        preview !== 'next-line' &&
        (!previewHidden || previewLeaving)
          ? 'grid-cols-[minmax(0,max-content)_minmax(min(4em,40%),1fr)] items-baseline gap-x-2'
          : 'grid-cols-1 gap-y-1'
      "
    >
      <span class="min-w-0 truncate">{{ title }}</span>
      <Transition
        enter-active-class="transition-[opacity,transform] duration-100 ease-out motion-reduce:transition-none"
        enter-from-class="translate-y-1 opacity-0"
        leave-active-class="transition-[opacity,transform] duration-100 ease-out motion-reduce:transition-none"
        leave-to-class="translate-y-1 opacity-0"
        @after-leave="previewLeaving = false"
        @leave-cancelled="previewLeaving = false"
      >
        <span
          v-if="contentPreview && preview !== 'hidden' && !previewHidden"
          class="min-w-0 truncate text-xs font-normal text-muted"
          >{{ contentPreview }}</span
        >
      </Transition>
    </span>
    <span v-if="categoryLabel" class="flex items-start gap-1.5 text-xs font-normal text-muted">
      <UIcon class="mt-0.5 size-3.5 shrink-0" name="i-lucide-folder" />
      <span class="min-w-0 truncate">{{ categoryLabel }}</span>
    </span>
  </span>
</template>
