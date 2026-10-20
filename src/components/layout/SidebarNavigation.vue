<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui';
import SidebarItem from './SidebarItem.vue';

defineProps<{ items: NavigationMenuItem[]; collapsed: boolean }>();
</script>

<template>
  <nav class="sidebar-navigation w-full px-2">
    <ul>
      <li v-for="item in items" :key="item.label">
        <SidebarItem
          :active="item.active"
          :collapsed="collapsed"
          exact
          :icon="item.icon!"
          :label="item.label"
          :to="item.to"
          @click="item.onSelect?.($event)"
        />
      </li>
    </ul>
  </nav>
</template>
