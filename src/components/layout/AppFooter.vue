<script setup lang="ts">
import Lark from '@/components/icons/Lark.vue';
import LogicalByteLogoSolid from '@/components/icons/LogicalByteLogoSolid.vue';
import Nga from '@/components/icons/Nga.vue';
import Skland from '@/components/icons/Skland.vue';
import { Component } from 'vue';

interface FooterLink {
  icon: string | Component;
  label: string;
  to: string;
}

const leftLinks: FooterLink[] = [
  {
    icon: 'i-simple-icons:rust',
    label: 'Rust',
    to: 'https://rust-lang.org/',
  },
  {
    icon: 'i-simple-icons:tauri',
    label: 'Tauri',
    to: 'https://tauri.app/',
  },
  {
    icon: 'i-simple-icons:vuedotjs',
    label: 'Vue.js',
    to: 'https://vuejs.org/',
  },
  {
    icon: 'i-simple-icons:nuxtdotjs',
    label: 'Nuxt UI',
    to: 'https://ui.nuxt.com/',
  },
  {
    icon: 'i-simple-icons:tailwindcss',
    label: 'Tailwind CSS',
    to: 'https://tailwindcss.com/',
  },
];

const rightLinks: FooterLink[] = [
  {
    icon: 'i-simple-icons:bilibili',
    label: '哔哩哔哩',
    to: 'https://space.bilibili.com/37179776',
  },
  {
    icon: Skland,
    label: '森空岛',
    to: 'https://www.skland.com/profile?id=1138552273321',
  },
  {
    icon: Nga,
    label: 'NGA',
    to: 'https://bbs.nga.cn/thread.php?authorid=60824354',
  },
  {
    icon: 'i-simple-icons:qq',
    label: 'QQ',
    to: 'https://qm.qq.com/q/5Lo35ydL0I',
  },
  {
    icon: Lark,
    label: '飞书',
    to: 'https://my.feishu.cn/wiki/V5VcwWMkeiz64mkuL9Vcot7unqd',
  },
  {
    icon: 'i-simple-icons:github',
    label: 'GitHub',
    to: 'https://github.com/BiologyHazard/riic-yituliu',
  },
];
</script>

<template>
  <USeparator :icon="LogicalByteLogoSolid" />

  <UFooter>
    <template #left>
      <p class="text-sm text-muted">Powered by</p>
      <UTooltip v-for="({ icon, label, to }, index) in leftLinks" :key="index" :text="label">
        <UButton
          :aria-label="label"
          color="neutral"
          :icon
          rel="noopener noreferrer"
          target="_blank"
          :to
          variant="ghost"
        />
      </UTooltip>
    </template>

    <template #right>
      <UTooltip v-for="({ icon, label, to }, index) in rightLinks" :key="index" :text="label">
        <UButton
          :aria-label="label"
          color="neutral"
          :icon
          rel="noopener noreferrer"
          target="_blank"
          :to
          variant="ghost"
        />
      </UTooltip>
    </template>
  </UFooter>
</template>
