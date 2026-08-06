<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui';

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
    label: '终末地一图流',
    icon: 'i-mdi-numeric-1-box-outline',
    to: 'https://ef.yituliu.cn/',
    target: '_blank',
  },
  {
    label: '终末地地图集',
    icon: 'i-lucide-map',
    to: 'https://oem.re/',
    target: '_blank',
  },
];
</script>

<template>
  <UHeader>
    <template #title>OEA</template>

    <UNavigationMenu :items="navigationMenuItems" />

    <template #body>
      <UNavigationMenu :items="navigationMenuItems" orientation="vertical" />
    </template>

    <template #right>
      <div class="flex items-center gap-1">
        <UTooltip text="切换颜色模式">
          <UColorModeButton />
        </UTooltip>
      </div>
    </template>
  </UHeader>
</template>
