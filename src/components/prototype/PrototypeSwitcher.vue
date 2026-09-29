<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const variants = [
  { key: 'a', label: 'A · 行内状态' },
  { key: 'b', label: 'B · 卡片状态栏' },
  { key: 'c', label: 'C · 紧凑目录' },
] as const;
const states = [
  { key: 'loading', label: '加载中' },
  { key: 'load-error', label: '加载失败' },
  { key: 'unsupported', label: '浏览器不支持' },
  { key: 'ready', label: '已就绪' },
] as const;

const route = useRoute();
const router = useRouter();
const isVisible = import.meta.env.DEV;

function currentVariantIndex(): number {
  const index = variants.findIndex(({ key }) => key === route.query.variant);
  return index >= 0 ? index : 0;
}

function updateQuery(patch: Record<string, string>): void {
  void router.replace({ query: { ...route.query, ...patch } });
}

function cycleVariant(offset: number): void {
  const index = (currentVariantIndex() + offset + variants.length) % variants.length;
  updateQuery({ variant: variants[index].key });
}

function onKeydown(event: KeyboardEvent): void {
  const target = event.target;
  if (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  ) {
    return;
  }
  if (event.key === 'ArrowLeft') {
    cycleVariant(-1);
  } else if (event.key === 'ArrowRight') {
    cycleVariant(1);
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <div
    v-if="isVisible"
    class="fixed bottom-5 left-1/2 z-50 flex max-w-[calc(100vw-2rem)] -translate-x-1/2 flex-wrap items-center justify-center gap-2 rounded-2xl border border-inverted/10 bg-inverted px-3 py-2 text-inverted shadow-2xl"
  >
    <span class="px-1 text-xs font-semibold tracking-wide uppercase opacity-70">Prototype</span>
    <div class="flex items-center gap-1 rounded-xl bg-default/10 p-1">
      <UButton
        aria-label="上一个方案"
        color="neutral"
        icon="i-lucide-chevron-left"
        size="xs"
        variant="ghost"
        @click="cycleVariant(-1)"
      />
      <span class="min-w-28 text-center text-xs font-medium">
        {{ variants[currentVariantIndex()].label }}
      </span>
      <UButton
        aria-label="下一个方案"
        color="neutral"
        icon="i-lucide-chevron-right"
        size="xs"
        variant="ghost"
        @click="cycleVariant(1)"
      />
    </div>
    <div class="flex flex-wrap justify-center gap-1">
      <UButton
        v-for="state in states"
        :key="state.key"
        color="neutral"
        :label="state.label"
        size="xs"
        :variant="
          route.query.state === state.key || (!route.query.state && state.key === 'loading')
            ? 'solid'
            : 'ghost'
        "
        @click="updateQuery({ state: state.key })"
      />
    </div>
    <UButton
      color="neutral"
      :icon="route.query.toast === 'on' ? 'i-lucide-bell-ring' : 'i-lucide-bell-off'"
      :label="route.query.toast === 'on' ? '全局提示：开' : '全局提示：关'"
      size="xs"
      variant="ghost"
      @click="updateQuery({ toast: route.query.toast === 'on' ? 'off' : 'on' })"
    />
  </div>
</template>
