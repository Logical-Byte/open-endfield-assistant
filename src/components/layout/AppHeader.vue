<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui';

const navigationMenuItems: NavigationMenuItem[] = [
  { label: '档案扫描', to: '/' },
  { label: '日志', to: '/log' },
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
