<script setup lang="ts" generic="T extends { id?: string; scannedItemId?: number }">
import { computed, ref, type ComponentPublicInstance } from 'vue';
import { useResizeObserver } from '@vueuse/core';
import {
  useVirtualizer,
  type VirtualItem,
  type Virtualizer,
  type VirtualizerOptions,
} from '@tanstack/vue-virtual';

const props = defineProps<{ items: readonly T[]; archiveRows?: boolean }>();
const root = ref<HTMLElement | null>(null);
const compact = ref(false);
// 卡片按实际内容排版，只测量实际改变高度的条目。
// 保留其他宽度变化中的测量缓存，避免退回估算高度后再补偿滚动。
useResizeObserver(root, (entries: readonly ResizeObserverEntry[]): void => {
  compact.value = entries[0]!.contentRect.width <= 480;
});
const virtualizer = useVirtualizer(
  computed(
    (): Omit<
      VirtualizerOptions<HTMLElement, Element>,
      'observeElementRect' | 'observeElementOffset' | 'scrollToFn'
    > => ({
      count: props.items.length,
      getScrollElement: (): HTMLElement | null => root.value,
      estimateSize: (): number => (props.archiveRows ? (compact.value ? 69 : 44) : 320),
      getItemKey: (index: number): string | number =>
        props.items[index]!.id ?? props.items[index]!.scannedItemId!,
      overscan: 8,
      gap: props.archiveRows ? 0 : 6,
      paddingStart: props.archiveRows ? 0 : 8,
      paddingEnd: props.archiveRows ? 0 : 8,
    }),
  ),
);
// 只补偿完全位于视口上方的条目，当前阅读行改变高度时保持其顶部偏移。
if (props.archiveRows) {
  virtualizer.value.shouldAdjustScrollPositionOnItemSizeChange = (
    item: VirtualItem,
    _delta: number,
    instance: Virtualizer<HTMLElement, Element>,
  ): boolean => item.end <= (instance.scrollOffset ?? 0);
}
const rows = computed((): VirtualItem[] => virtualizer.value.getVirtualItems());
const total = computed((): number => virtualizer.value.getTotalSize());
function measure(element: Element | ComponentPublicInstance | null): void {
  if (element === null || element instanceof Element) {
    virtualizer.value.measureElement(element);
  }
}
defineExpose({ virtualizer });
</script>

<template>
  <div
    ref="root"
    class="group/archive overflow-y-auto [overflow-anchor:none]"
    :data-compact="compact"
  >
    <div class="relative w-full" data-slot="viewport" :style="{ height: `${total}px` }">
      <div
        v-for="row in rows"
        :key="String(row.key)"
        :ref="measure"
        class="absolute top-0 left-0 w-full"
        :data-index="row.index"
        :style="{ transform: `translateY(${row.start}px)` }"
      >
        <slot :item="items[row.index]!" />
      </div>
    </div>
  </div>
</template>
