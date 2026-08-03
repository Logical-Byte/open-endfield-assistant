<script setup lang="ts">
import Lark from '@/components/icons/Lark.vue';
import LogicalByteLogoSolid from '@/components/icons/LogicalByteLogoSolid.vue';
import Nga from '@/components/icons/Nga.vue';
import Skland from '@/components/icons/Skland.vue';
</script>

<template>
  <USeparator :icon="LogicalByteLogoSolid" />

  <UFooter>
    <template #left>
      <p class="text-sm text-muted">Powered by</p>
      <UButton
        aria-label="Rust"
        color="neutral"
        icon="i-simple-icons:rust"
        rel="noopener noreferrer"
        target="_blank"
        to="https://rust-lang.org/"
        variant="ghost"
      />
      <UButton
        aria-label="Tauri"
        color="neutral"
        icon="i-simple-icons:tauri"
        rel="noopener noreferrer"
        target="_blank"
        to="https://tauri.app/"
        variant="ghost"
      />
      <UButton
        aria-label="Vue.js"
        color="neutral"
        icon="i-simple-icons:vuedotjs"
        rel="noopener noreferrer"
        target="_blank"
        to="https://vuejs.org/"
        variant="ghost"
      />
      <UButton
        aria-label="Nuxt UI"
        color="neutral"
        icon="i-simple-icons:nuxtdotjs"
        rel="noopener noreferrer"
        target="_blank"
        to="https://ui.nuxt.com/"
        variant="ghost"
      />
      <UButton
        aria-label="Tailwind CSS"
        color="neutral"
        icon="i-simple-icons:tailwindcss"
        rel="noopener noreferrer"
        target="_blank"
        to="https://tailwindcss.com/"
        variant="ghost"
      />
    </template>

    <template #right>
      <UButton
        aria-label="哔哩哔哩"
        color="neutral"
        icon="i-simple-icons:bilibili"
        rel="noopener noreferrer"
        target="_blank"
        to="https://space.bilibili.com/37179776"
        variant="ghost"
      />
      <UButton
        aria-label="森空岛"
        color="neutral"
        :icon="Skland"
        rel="noopener noreferrer"
        target="_blank"
        to="https://www.skland.com/profile?id=1138552273321"
        variant="ghost"
      />
      <UButton
        aria-label="NGA"
        color="neutral"
        :icon="Nga"
        rel="noopener noreferrer"
        target="_blank"
        to="https://bbs.nga.cn/thread.php?authorid=60824354"
        variant="ghost"
      />
      <UButton
        aria-label="QQ"
        color="neutral"
        icon="i-simple-icons:qq"
        rel="noopener noreferrer"
        target="_blank"
        to="https://qm.qq.com/q/5Lo35ydL0I"
        variant="ghost"
      />
      <UButton
        aria-label="飞书"
        color="neutral"
        :icon="Lark"
        rel="noopener noreferrer"
        target="_blank"
        to="https://my.feishu.cn/wiki/V5VcwWMkeiz64mkuL9Vcot7unqd"
        variant="ghost"
      />
      <UButton
        aria-label="GitHub"
        color="neutral"
        icon="i-simple-icons:github"
        rel="noopener noreferrer"
        target="_blank"
        to="https://github.com/BiologyHazard/riic-yituliu"
        variant="ghost"
      />
    </template>
  </UFooter>
</template>
