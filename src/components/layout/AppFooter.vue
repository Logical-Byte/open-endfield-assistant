<script setup lang="ts">
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
    to: 'https://space.bilibili.com/688411531',
  },
  {
    icon: Skland,
    label: '森空岛',
    to: 'https://www.skland.com/profile?id=3679911155887',
  },
  {
    icon: Nga,
    label: 'NGA',
    to: 'https://bbs.nga.cn/thread.php?authorid=60824354',
  },
  {
    icon: 'i-simple-icons:tiktok',
    label: '抖音',
    to: 'https://www.douyin.com/user/MS4wLjABAAAAObl3owR4oOSC-OPUBF_LhqnVLdVzzSDMvgcxBiYqjTIDaIKML95YRyBY_fPpRMUy',
  },
  {
    icon: 'i-simple-icons:qq',
    label: 'QQ',
    to: 'https://qm.qq.com/cgi-bin/qm/qr?k=WBgxAB_VBXczwq-4GsS3iLS7QIOfhcZx',
  },
  {
    icon: 'i-simple-icons:github',
    label: 'GitHub',
    to: 'https://github.com/Logical-Byte/open-endfield-assistant',
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
