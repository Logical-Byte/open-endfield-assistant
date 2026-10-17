<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { appPresentation } from '@/app/presentation';
import { AppVariant } from '@/app/build';
import { updateCapabilities } from '@/features/update/variantPolicy';
import { computed } from 'vue';
import { NavigationMenuItem } from '@nuxt/ui';

const { t } = useAppI18n();
const navigationMenuItems = computed<NavigationMenuItem[]>(() => [
  {
    label: t('navigation.scan'),
    icon: 'i-lucide-scan-line',
    to: '/',
  },
  {
    label: t('navigation.logs'),
    icon: 'i-lucide-scroll-text',
    to: '/log',
  },
  {
    label: t('navigation.monitor'),
    icon: 'i-lucide-monitor-play',
    to: '/monitor',
  },
  {
    label: t('navigation.help'),
    icon: 'i-lucide-circle-help',
    to: '/help',
  },
]);
</script>

<template>
  <UHeader
    :menu="{ title: t('navigation.menu.title'), description: t('navigation.menu.description') }"
    :ui="{
      title: 'inline',
      left: 'lg:flex-1',
      center: 'hidden lg:flex',
      right: 'lg:flex-1',
      toggle: 'lg:hidden',
      content: 'lg:hidden',
      overlay: 'lg:hidden',
    }"
  >
    <template #title>
      <span>{{ appPresentation.name }}</span
      ><span
        v-if="appPresentation.variant === AppVariant.Release"
        class="text-base font-medium text-muted"
      >
        v{{ appPresentation.version }}</span
      >
    </template>

    <UNavigationMenu :items="navigationMenuItems" />

    <template #body>
      <UNavigationMenu :items="navigationMenuItems" orientation="vertical" />
    </template>

    <template #right>
      <div class="flex items-center gap-1">
        <UTooltip :text="t('settings.language.shortcut')">
          <UButton
            :aria-label="t('settings.language.shortcut')"
            class="text-muted"
            color="neutral"
            icon="i-lucide-languages"
            to="/settings#language"
            variant="ghost"
          />
        </UTooltip>
        <UpdatePopover v-if="updateCapabilities.onlineUpdate" />
        <UTooltip :text="t('navigation.settings')">
          <UButton
            :aria-label="t('navigation.settings')"
            class="text-muted"
            color="neutral"
            icon="i-lucide-settings"
            to="/settings#interface"
            :variant="$route.path === '/settings' ? 'soft' : 'ghost'"
          />
        </UTooltip>
      </div>
    </template>
  </UHeader>
</template>
