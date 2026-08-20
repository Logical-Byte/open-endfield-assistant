<script setup lang="ts">
import { oeaVersion } from '@/main';
import { NavigationMenuItem } from '@nuxt/ui';

const navigationMenuItems: NavigationMenuItem[] = [
  {
    label: '档案扫描',
    icon: 'i-lucide-scan-line',
    to: '/',
  },
  {
    label: '日志',
    icon: 'i-lucide-scroll-text',
    to: '/log',
  },
  {
    label: '监控',
    icon: 'i-lucide-monitor-play',
    to: '/monitor',
  },
  {
    label: '设置',
    icon: 'i-lucide-settings',
    to: '/settings',
  },
  {
    label: '帮助',
    icon: 'i-lucide-circle-help',
    to: '/help',
  },
  {
    label: '一图流',
    icon: 'i-mdi-numeric-1-box-outline',
    to: 'https://ef.yituliu.cn/',
    target: '_blank',
  },
  {
    label: '地图集',
    icon: 'i-lucide-map',
    to: 'https://oem.re/',
    target: '_blank',
  },
];
</script>

<template>
  <UHeader
    :ui="{
      title: 'inline',
      left: 'md:flex-1',
      center: 'hidden md:flex',
      right: 'md:flex-1',
      toggle: 'md:hidden',
      content: 'md:hidden',
      overlay: 'md:hidden',
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
        <UpdatePopover />
        <ThemePicker />
        <UTooltip text="切换颜色模式">
          <UColorModeButton />
        </UTooltip>
      </div>
    </template>
  </UHeader>
</template>
