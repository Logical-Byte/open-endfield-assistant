<script setup lang="ts">
import logoUrl from '@/assets/images/白鸥.webp';
import SklandOopa from '@/components/icons/SklandOopa.vue';
import type { NavigationMenuItem } from '@nuxt/ui';
import { computed } from 'vue';

const collapsed = defineModel<boolean>('collapsed');

const itemsExpanded: NavigationMenuItem[] = [
  {
    label: '首页',
    icon: 'i-lucide-home',
    to: '/',
  },
  {
    label: '明日方舟',
    icon: 'i-lucide-shield-plus',
    defaultOpen: true,
    children: [
      {
        label: '罗德岛基建',
        icon: 'i-lucide-factory',
        defaultOpen: true,
        children: [
          {
            label: '排班表生成器',
            icon: 'i-lucide-calendar-sync',
            to: '/riic',
          },
          {
            label: '基建技能',
            icon: 'i-lucide-wrench',
            to: '/base-skill',
          },
          {
            label: '干员基建技能展示',
            icon: 'i-lucide-presentation',
            to: '/operator-base-skill-showcase',
          },
          {
            label: '干员头像生成器',
            icon: 'i-lucide-user-round-pen',
            to: '/operator-avatar-generator',
          },
          {
            label: '基建地图',
            icon: 'i-lucide-map',
            to: '/riic-map',
          },
        ],
      },
      {
        label: '干员养成',
        icon: 'i-lucide-trending-up',
        defaultOpen: true,
        children: [
          {
            label: '干员拉满消耗',
            icon: 'i-lucide-arrow-up-to-line',
            to: '/char-item-cost',
          },
          {
            label: '养成成本排行',
            icon: 'i-lucide-list-ordered',
            to: '/char-cost-ranking',
          },
        ],
      },
      {
        label: '罗德岛物价局',
        icon: 'i-lucide-shopping-bag',
        defaultOpen: true,
        children: [
          {
            label: '材料信息',
            icon: 'i-lucide-boxes',
            to: '/material-info',
          },
          {
            label: '物品价值',
            icon: 'i-lucide-coins',
            to: '/item-value',
          },
        ],
      },
      {
        label: '作战与情报',
        icon: 'i-lucide-newspaper',
        defaultOpen: true,
        children: [
          {
            label: '作战列表',
            icon: 'i-lucide-swords',
            to: '/stages',
          },
          {
            label: '明日方舟游戏内公告',
            icon: 'i-lucide-megaphone',
            to: '/arknights-game-bulletin',
          },
          {
            label: '塞壬唱片',
            icon: 'i-lucide-disc-3',
            to: '/monster-siren',
          },
          {
            label: '明日方舟一图流',
            icon: 'i-mdi-numeric-1-box-outline',
            to: 'https://ark.yituliu.cn/',
            target: '_blank',
          },
        ],
      },
    ],
  },
  {
    label: '明日方舟终末地',
    icon: 'i-lucide-satellite',
    defaultOpen: true,
    children: [
      {
        label: '终末地游戏内公告',
        icon: 'i-lucide-megaphone',
        to: '/endfield-game-bulletin',
      },
      {
        label: '终末地一图流',
        icon: 'i-mdi-numeric-1-box-outline',
        to: 'https://ef.yituliu.cn/',
        target: '_blank',
      },
    ],
  },
  {
    label: '工作室头像生成器',
    icon: 'i-lucide-image',
    to: '/studio-avatar',
  },
  {
    label: '图片渐变工具',
    icon: 'i-lucide-blend',
    to: '/image-gradient',
  },
  {
    label: 'BioBot 森空岛小助手',
    icon: SklandOopa,
    to: '/sklassistant',
  },
  {
    label: '友情链接',
    icon: 'i-lucide-link',
    to: '/links',
  },
];

function flattenNavigationItems(items: NavigationMenuItem[]): NavigationMenuItem[] {
  const result: NavigationMenuItem[] = [];
  for (const item of items) {
    if (item.children) {
      result.push(...flattenNavigationItems(item.children));
    } else {
      result.push(item);
    }
  }
  return result;
}

const itemsCollapsed = computed<NavigationMenuItem[]>(() => flattenNavigationItems(itemsExpanded));
</script>

<template>
  <UDashboardSidebar
    v-model:collapsed="collapsed"
    auto-close
    class="group/sidebar border-none py-4 transition-none duration-300 data-[dragging=false]:transition-[width]"
    collapsible
    :default-size="16"
    :max-size="24"
    :min-size="12"
    :persistent="false"
    resizable
    side="left"
    :ui="{
      overlay: 'lg:block',
      body: 'scrollbar-thin group-data-[collapsed=true]/sidebar:scrollbar-none',
      content:
        'fixed inset-y-4 left-4 flex w-[calc(100%-(--spacing(8)))] rounded-lg ring-default sm:shadow-lg sm:ring lg:flex',
    }"
  >
    <template #header>
      <div class="flex min-w-0 items-center justify-start gap-2">
        <UButton
          :avatar="{
            src: logoUrl,
            size: 'xs',
            alt: 'Logo',
            class: 'rounded-none bg-transparent',
          }"
          class="shrink-0 p-1.5"
          to="/"
          variant="ghost"
        />
        <div v-if="!collapsed" class="truncate font-bold">明日方舟基建一图流</div>
      </div>
    </template>

    <UNavigationMenu
      :collapsed="collapsed"
      :items="collapsed ? itemsCollapsed : itemsExpanded"
      orientation="vertical"
      :tooltip="true"
      :ui="{
        link: 'p-1.5 text-toned data-active:text-primary',
        linkLeadingIcon: 'text-toned group-data-active:text-primary',
      }"
      variant="pill"
    />
  </UDashboardSidebar>
</template>
