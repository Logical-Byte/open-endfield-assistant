<script setup lang="ts">
import { computed, ref } from 'vue';
import type { NavigationMenuItem } from '@nuxt/ui';
import { appPresentation } from '@/app/presentation';
import { AppVariant } from '@/app/build';
import { useAppI18n } from '@/shared/i18n';
import { updateCapabilities } from '@/features/update/variantPolicy';
import SidebarNavigation from './SidebarNavigation.vue';
import SidebarItem from './SidebarItem.vue';
import { useColorMode } from '@vueuse/core';

const { t } = useAppI18n();
const colorMode = useColorMode();
function toggleColorMode(): void {
  colorMode.value = colorMode.value === 'dark' ? 'light' : 'dark';
}
const faviconUrl = '/favicon.ico';
const collapsedWidth = 56;
const minWidth = 144;
const maxWidth = 280;
const defaultWidth = 176;
const collapsed = ref(false);
const expandedWidth = ref(defaultWidth);
const width = computed<number>(() => (collapsed.value ? collapsedWidth : expandedWidth.value));
const dragging = ref(false);
let dragOriginX = 0;
let dragOriginWidth = defaultWidth;

function startDrag(event: PointerEvent): void {
  if (event.button !== 0) return;
  event.preventDefault();
  dragOriginX = event.clientX;
  dragOriginWidth = width.value;
  dragging.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function moveDrag(event: PointerEvent): void {
  if (!dragging.value) return;
  const nextWidth = dragOriginWidth + event.clientX - dragOriginX;
  // 最小展开宽度为 144px，再向左拖 32px 收起。展开阈值保留回差，避免边界抖动。
  if (collapsed.value) {
    if (nextWidth < minWidth - 16) return;
    collapsed.value = false;
  } else if (nextWidth < minWidth - 32) {
    collapsed.value = true;
    return;
  }
  expandedWidth.value = Math.min(maxWidth, Math.max(minWidth, nextWidth));
}

function stopDrag(): void {
  dragging.value = false;
}

function resizeWithKeyboard(event: KeyboardEvent): void {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  if (collapsed.value) {
    if (event.key === 'ArrowRight') collapsed.value = false;
  } else if (event.key === 'ArrowLeft' && expandedWidth.value === minWidth) {
    collapsed.value = true;
  } else {
    expandedWidth.value = Math.min(
      maxWidth,
      Math.max(minWidth, expandedWidth.value + (event.key === 'ArrowLeft' ? -8 : 8)),
    );
  }
}

function resetWidth(): void {
  collapsed.value = false;
  expandedWidth.value = defaultWidth;
}

const navigationItems = computed<NavigationMenuItem[]>(() => [
  { label: t('navigation.scan'), icon: 'i-lucide-scan-line', to: '/', exact: true },
  { label: t('navigation.logs'), icon: 'i-lucide-scroll-text', to: '/log' },
  { label: t('navigation.monitor'), icon: 'i-lucide-monitor-play', to: '/monitor' },
  { label: t('navigation.help'), icon: 'i-lucide-circle-help', to: '/help' },
]);
const settingsItems = computed<NavigationMenuItem[]>(() => [
  { label: t('navigation.settings'), icon: 'i-lucide-settings', to: '/settings#interface' },
]);
</script>

<template>
  <UDashboardSidebar
    id="app"
    v-model:collapsed="collapsed"
    :aria-label="t('navigation.menu.title')"
    class="app-sidebar relative m-2 mr-0 flex! h-auto! min-h-0 min-w-0 shrink-0 self-stretch rounded-xl border-r-0 bg-elevated shadow-md ring shadow-black/15 ring-accented ring-inset dark:bg-muted"
    :collapsed-size="collapsedWidth"
    collapsible
    :default-size="defaultWidth"
    :max-size="maxWidth"
    :min-size="minWidth"
    :style="{ width: `${width}px` }"
    :toggle="false"
    :ui="{ body: 'flex min-h-0 flex-1 flex-col gap-0 overflow-x-hidden p-0' }"
  >
    <template #resize-handle>
      <UDashboardResizeHandle
        aria-controls="dashboard-sidebar-app"
        :aria-label="t('navigation.sidebar.resize')"
        aria-orientation="vertical"
        :aria-valuemax="maxWidth"
        :aria-valuemin="collapsedWidth"
        :aria-valuenow="width"
        :aria-valuetext="
          collapsed ? t('navigation.sidebar.collapsed') : t('navigation.sidebar.width', { width })
        "
        class="z-10 -mx-1 block! w-2 shrink-0 touch-none hover:bg-transparent focus-visible:bg-accented focus-visible:outline-none"
        tabindex="0"
        :title="t('navigation.sidebar.resizeHint')"
        @dblclick="resetWidth"
        @keydown="resizeWithKeyboard"
        @lostpointercapture="stopDrag"
        @pointercancel="stopDrag"
        @pointerdown="startDrag"
        @pointermove="moveDrag"
        @pointerup="stopDrag"
      />
    </template>
    <div class="flex h-14 shrink-0 items-center px-2" :title="appPresentation.name">
      <div class="flex w-10 shrink-0 items-center justify-center">
        <img alt="OEA" class="size-6 shrink-0 object-contain" draggable="false" :src="faviconUrl" />
      </div>
      <div
        class="flex min-w-0 items-baseline gap-1.5 whitespace-nowrap"
        :class="collapsed && 'invisible'"
      >
        <span class="text-sm font-semibold text-toned">OEA</span>
        <span class="text-xs text-muted">{{
          appPresentation.variant === AppVariant.Dev ? 'dev' : `v${appPresentation.version}`
        }}</span>
      </div>
    </div>
    <SidebarNavigation :collapsed="collapsed" :items="navigationItems" />
    <div class="mt-auto flex shrink-0 flex-col items-start pb-2">
      <div class="w-full px-2">
        <UpdatePopover v-if="updateCapabilities.onlineUpdate" :collapsed="collapsed" />
        <SidebarItem
          :collapsed="collapsed"
          :icon="colorMode === 'dark' ? 'i-lucide-moon' : 'i-lucide-sun'"
          :tooltip="t('theme.toggleMode')"
          @click="toggleColorMode"
        />
        <SidebarItem
          :active="false"
          :collapsed="collapsed"
          icon="i-lucide-languages"
          to="/settings#language"
          :tooltip="t('settings.language.shortcut')"
        />
      </div>
      <SidebarNavigation :collapsed="collapsed" :items="settingsItems" />
    </div>
  </UDashboardSidebar>
</template>
