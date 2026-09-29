<script setup lang="ts">
import PrototypeSwitcher from '@/components/prototype/PrototypeSwitcher.vue';
import SettingsStateVariantA from '@/components/prototype/settings/SettingsStateVariantA.vue';
import SettingsStateVariantB from '@/components/prototype/settings/SettingsStateVariantB.vue';
import SettingsStateVariantC from '@/components/prototype/settings/SettingsStateVariantC.vue';
import { computed } from 'vue';
import { useRoute } from 'vue-router';

// 临时视觉原型：通过 ?variant= 比较三种 Settings 生命周期界面。
// 结论：采用 A 的行内状态。加载失败的全局提示仅在 Settings 页面以外显示。
const route = useRoute();
const variant = computed(() =>
  ['a', 'b', 'c'].includes(String(route.query.variant)) ? String(route.query.variant) : 'a',
);
const state = computed(() =>
  ['loading', 'load-error', 'unsupported', 'ready'].includes(String(route.query.state))
    ? String(route.query.state)
    : 'loading',
);

const sections = [
  { id: 'interface', icon: 'i-lucide-layout-panel-left', title: '界面设置' },
  { id: 'sound', icon: 'i-lucide-headphones', title: '声音设置' },
  { id: 'update', icon: 'i-lucide-download', title: '更新设置' },
  { id: 'developer', icon: 'i-lucide-code-2', title: '开发者选项' },
];
</script>

<template>
  <UContainer class="min-h-[calc(100lvh-var(--ui-header-height)-var(--ui-title-height))] pb-28">
    <UPage>
      <template #left>
        <UPageAside :ui="{ root: 'lg:sticky lg:top-0 lg:self-start' }">
          <div
            class="mb-4 rounded-xl border border-warning/30 bg-warning/5 p-3 text-xs text-dimmed"
          >
            <p class="font-semibold text-highlighted">Throwaway prototype</p>
            <p class="mt-1">只比较首次加载与不可用状态，不连接真实后端。</p>
          </div>
          <nav class="flex flex-col gap-1">
            <UButton
              v-for="section in sections"
              :key="section.id"
              class="w-full justify-start"
              color="neutral"
              :icon="section.icon"
              :label="section.title"
              :to="`#${section.id}`"
              variant="ghost"
            />
          </nav>
        </UPageAside>
      </template>

      <UPageBody>
        <div class="mb-2">
          <p class="text-sm font-medium text-primary">Settings lifecycle prototype</p>
          <h1 class="mt-1 text-2xl font-semibold text-highlighted">
            没有前端默认值时，设置页应该如何呈现？
          </h1>
          <p class="mt-2 max-w-3xl text-sm text-dimmed">
            使用页面底部的工具栏切换方案和状态。方向键可以快速切换 A、B、C。
          </p>
        </div>

        <SettingsStateVariantA v-if="variant === 'a'" :state="state" />
        <SettingsStateVariantB v-else-if="variant === 'b'" :state="state" />
        <SettingsStateVariantC v-else :state="state" />
      </UPageBody>
    </UPage>
  </UContainer>

  <Transition
    enter-active-class="transition duration-200"
    enter-from-class="translate-y-2 opacity-0"
    leave-active-class="transition duration-150"
    leave-to-class="translate-y-2 opacity-0"
  >
    <div
      v-if="state === 'load-error' && route.query.toast === 'on'"
      class="fixed top-20 right-5 z-40 w-96 max-w-[calc(100vw-2rem)] rounded-xl border border-error/25 bg-default p-4 shadow-xl"
    >
      <div class="flex gap-3">
        <UIcon class="mt-0.5 text-xl text-error" name="i-lucide-circle-alert" />
        <div class="flex-1">
          <p class="font-semibold">设置加载失败</p>
          <p class="mt-1 text-sm text-dimmed">自动更新和扫描提示暂时不会使用用户设置。</p>
          <UButton class="mt-3" color="error" label="前往设置" size="sm" variant="soft" />
        </div>
      </div>
    </div>
  </Transition>

  <PrototypeSwitcher />
</template>
