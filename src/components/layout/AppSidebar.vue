<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue';
import { useRoute } from 'vue-router';
import {
  activeHelpSectionId,
  helpSectionRequest,
  helpSections,
  settingsSections,
} from '@/app/sectionNavigation';
import type { NavigationMenuItem } from '@nuxt/ui';
import { appPresentation } from '@/app/presentation';
import { AppVariant } from '@/app/build';
import { useAppI18n } from '@/shared/i18n';
import { updateCapabilities } from '@/features/update/variantPolicy';
import { updatePopoverOpen } from '@/features/update/updatePopover';
import SidebarNavigation from './SidebarNavigation.vue';
import SidebarItem from './SidebarItem.vue';
import { useColorMode, useEventListener } from '@vueuse/core';

const { t } = useAppI18n();
const colorMode = useColorMode();
const route = useRoute();
const sectionPage = computed(() => route.path === '/help' || route.path === '/settings');
const returnTo = ref('/');
// 只记录主页面，分类间切换不改变返回目标，也不触碰侧栏的宽度状态。
watch(
  () => route.fullPath,
  (path: string): void => {
    if (!sectionPage.value) returnTo.value = path;
  },
  { immediate: true },
);
const sectionItems = computed<NavigationMenuItem[]>((): NavigationMenuItem[] => {
  if (route.path === '/settings') {
    const activeId = settingsSections.some((section) => `#${section.id}` === route.hash)
      ? route.hash.slice(1)
      : 'language';
    return settingsSections.map((section) => ({
      label: t(section.titleKey),
      icon: section.icon,
      to: { path: '/settings', hash: `#${section.id}` },
      active: activeId === section.id,
    }));
  }
  return helpSections.map((section) => ({
    label: t(section.titleKey),
    icon: section.icon,
    active: activeHelpSectionId.value === section.id,
    onSelect: (): void => {
      helpSectionRequest.value = { id: section.id };
    },
  }));
});
function toggleColorMode(): void {
  colorMode.value = colorMode.value === 'dark' ? 'light' : 'dark';
}
const faviconUrl = '/favicon.ico';
const collapsedWidth = 56;
const minWidth = 144;
const maxWidth = 280;
const defaultWidth = 176;
const pinned = ref(true);
const hovered = ref(false);
const sidebarArea = useTemplateRef<HTMLElement>('sidebarArea');
const keyboardFocusWithin = ref(false);
const expandedWidth = ref(defaultWidth);
const collapsed = computed<boolean>(
  (): boolean =>
    !pinned.value && !hovered.value && !keyboardFocusWithin.value && !updatePopoverOpen.value,
);
const width = computed<number>((): number =>
  collapsed.value ? collapsedWidth : expandedWidth.value,
);
// 布局占位立即切换；浮动侧栏自己的宽度单独过渡，不挤动右侧内容。
const reservedWidth = computed<number>((): number =>
  pinned.value ? expandedWidth.value : collapsedWidth,
);
const dragging = ref(false);
let dragOriginX = 0;
let dragOriginWidth = defaultWidth;

function handleMouseMove(event: MouseEvent): void {
  const bounds = sidebarArea.value?.getBoundingClientRect();
  if (!bounds) return;
  // 用坐标扩展悬停范围，避免透明覆盖层挡住工作区的点击和滚动。
  hovered.value =
    event.clientX >= bounds.left - 8 &&
    event.clientX <= bounds.left + width.value + 24 &&
    event.clientY >= bounds.top - 8 &&
    event.clientY <= bounds.bottom + 8;
}

useEventListener(window, 'mousemove', handleMouseMove);
useEventListener(document, 'mouseleave', (): void => {
  hovered.value = false;
});

function startDrag(event: PointerEvent): void {
  if (event.button !== 0) return;
  event.preventDefault();
  dragOriginX = event.clientX;
  dragOriginWidth = expandedWidth.value;
  dragging.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function moveDrag(event: PointerEvent): void {
  if (!dragging.value) return;
  const nextWidth = dragOriginWidth + event.clientX - dragOriginX;
  expandedWidth.value = Math.min(maxWidth, Math.max(minWidth, nextWidth));
}

function stopDrag(): void {
  dragging.value = false;
}

function resizeWithKeyboard(event: KeyboardEvent): void {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  expandedWidth.value = Math.min(
    maxWidth,
    Math.max(minWidth, expandedWidth.value + (event.key === 'ArrowLeft' ? -8 : 8)),
  );
}

function resetWidth(): void {
  expandedWidth.value = defaultWidth;
}

function handleFocusIn(event: FocusEvent): void {
  keyboardFocusWithin.value = (event.target as HTMLElement).matches(':focus-visible');
}

function handleFocusOut(event: FocusEvent): void {
  if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null)) {
    keyboardFocusWithin.value = false;
  }
}

const navigationItems = computed<NavigationMenuItem[]>(() => [
  { label: t('navigation.scan'), icon: 'i-lucide-scan-line', to: '/', exact: true },
  { label: t('navigation.logs'), icon: 'i-lucide-scroll-text', to: '/log' },
  { label: t('navigation.monitor'), icon: 'i-lucide-monitor-play', to: '/monitor' },
  { label: t('navigation.help'), icon: 'i-lucide-circle-help', to: '/help' },
]);
const settingsItems = computed<NavigationMenuItem[]>(() => [
  { label: t('navigation.settings'), icon: 'i-lucide-settings', to: '/settings' },
]);
</script>

<template>
  <div
    ref="sidebarArea"
    class="relative min-h-0 self-stretch"
    :style="{ width: `${reservedWidth}px` }"
    @focusin="handleFocusIn"
    @focusout="handleFocusOut"
    @pointerdown="keyboardFocusWithin = false"
  >
    <UDashboardSidebar
      id="app"
      :aria-label="t('navigation.menu.title')"
      class="app-sidebar absolute! inset-y-0 left-0 z-20 flex! h-full! min-h-0 min-w-0 overflow-hidden rounded-xl border-r-0 bg-elevated ring ring-accented ring-inset dark:bg-muted"
      :class="[
        { 'sidebar-resizing': dragging },
        !pinned && !collapsed
          ? 'shadow-xl shadow-black/25 dark:shadow-black/60 dark:ring-inverted/20'
          : 'shadow-md shadow-black/15',
      ]"
      :collapsed="collapsed"
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
          v-if="pinned"
          aria-controls="dashboard-sidebar-app"
          :aria-label="t('navigation.sidebar.resize')"
          aria-orientation="vertical"
          :aria-valuemax="maxWidth"
          :aria-valuemin="minWidth"
          :aria-valuenow="expandedWidth"
          :aria-valuetext="t('navigation.sidebar.width', { width: expandedWidth })"
          class="absolute! inset-y-0 -right-3 z-30 block! w-3 touch-none hover:bg-transparent focus-visible:bg-accented focus-visible:outline-none"
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
      <UTooltip :text="pinned ? t('navigation.sidebar.unpin') : t('navigation.sidebar.pin')">
        <UButton
          v-show="!collapsed"
          :aria-label="pinned ? t('navigation.sidebar.unpin') : t('navigation.sidebar.pin')"
          :aria-pressed="pinned"
          class="absolute top-4 z-10 text-muted hover:text-toned"
          color="neutral"
          icon="i-lucide-pin"
          size="xs"
          :style="{ left: `${expandedWidth - 32}px` }"
          :ui="{ leadingIcon: pinned ? '[&_path]:fill-current' : 'rotate-45' }"
          :variant="pinned ? 'subtle' : 'ghost'"
          @click="pinned = !pinned"
        />
      </UTooltip>
      <div class="relative min-h-0 flex-1 overflow-hidden">
        <Transition :name="sectionPage ? 'sidebar-forward' : 'sidebar-back'">
          <div :key="sectionPage ? route.path : 'main'" class="flex h-full min-h-0 w-full flex-col">
            <div
              v-if="sectionPage"
              class="flex h-14 shrink-0 items-center ps-2"
              :class="collapsed ? 'pe-2' : 'pe-10'"
            >
              <SidebarItem
                :collapsed="collapsed"
                icon="i-lucide-arrow-left"
                :label="t('navigation.back')"
                :to="returnTo"
              />
            </div>
            <div
              v-else
              class="flex h-14 shrink-0 items-center ps-2"
              :class="collapsed ? 'pe-2' : 'pe-10'"
              :title="appPresentation.name"
            >
              <div class="flex w-10 shrink-0 items-center justify-center">
                <img
                  alt="OEA"
                  class="size-6 shrink-0 object-contain"
                  draggable="false"
                  :src="faviconUrl"
                />
              </div>
              <div
                class="flex min-w-0 items-baseline gap-1.5 overflow-hidden whitespace-nowrap"
                :class="collapsed && 'invisible'"
              >
                <span class="shrink-0 text-sm font-semibold text-toned">OEA</span>
                <span class="truncate text-xs text-muted">{{
                  appPresentation.variant === AppVariant.Dev ? 'dev' : `v${appPresentation.version}`
                }}</span>
              </div>
            </div>
            <SidebarNavigation
              :collapsed="collapsed"
              :items="sectionPage ? sectionItems : navigationItems"
            />
            <div v-if="!sectionPage" class="mt-auto flex shrink-0 flex-col items-start pb-2">
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
          </div>
        </Transition>
      </div>
    </UDashboardSidebar>
  </div>
</template>

<style scoped>
:deep(.app-sidebar) {
  transition:
    width 220ms cubic-bezier(0.22, 1, 0.36, 1),
    box-shadow 220ms ease;
}

:deep(.app-sidebar.sidebar-resizing) {
  transition: none;
}

.sidebar-forward-enter-active,
.sidebar-forward-leave-active,
.sidebar-back-enter-active,
.sidebar-back-leave-active {
  transition:
    transform 220ms cubic-bezier(0.22, 1, 0.36, 1),
    opacity 220ms ease;
}

/* 离场内容叠在原位置，避免同时进出时挤动侧栏布局。 */
.sidebar-forward-leave-active,
.sidebar-back-leave-active {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.sidebar-forward-enter-from,
.sidebar-back-leave-to {
  opacity: 0;
  transform: translateX(32px);
}

.sidebar-forward-leave-to,
.sidebar-back-enter-from {
  opacity: 0;
  transform: translateX(-32px);
}

@media (prefers-reduced-motion: reduce) {
  :deep(.app-sidebar),
  .sidebar-forward-enter-active,
  .sidebar-forward-leave-active,
  .sidebar-back-enter-active,
  .sidebar-back-leave-active {
    transition: none;
  }
}
</style>
