<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { oeaVersion } from '@/version';
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
    label: t('navigation.settings'),
    icon: 'i-lucide-settings',
    to: '/settings',
  },
  {
    label: t('navigation.help'),
    icon: 'i-lucide-circle-help',
    to: '/help',
  },
  {
    label: t('navigation.yituliu'),
    icon: 'i-mdi-numeric-1-box-outline',
    to: 'https://ef.yituliu.cn/',
    target: '_blank',
  },
  {
    label: t('navigation.maps'),
    icon: 'i-lucide-map',
    to: 'https://oem.re/',
    target: '_blank',
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
      <span>OEA</span
      ><span v-if="oeaVersion" class="text-base font-medium text-muted"> v{{ oeaVersion }}</span>
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
            color="neutral"
            icon="i-lucide-languages"
            to="/settings#language"
            variant="ghost"
          />
        </UTooltip>
        <UpdatePopover />
        <ThemePicker />
        <UTooltip :text="t('theme.toggleMode')">
          <UColorModeButton />
        </UTooltip>
      </div>
    </template>
  </UHeader>
</template>
