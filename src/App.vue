<script setup lang="ts">
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed } from 'vue';

const colorMode = useColorMode();
const themeColor = computed(() => (colorMode.value === 'dark' ? '#18181b' : '#ffffff'));

useHead({
  meta: [{ name: 'theme-color', content: themeColor }],
});
</script>

<template>
  <Suspense>
    <UApp>
      <UHeader>
        <template #left>
          <p class="text-base font-semibold">OEA</p>
        </template>

        <template #right>
          <UColorModeButton />

          <UButton
            aria-label="GitHub"
            color="neutral"
            icon="simple-icons:github"
            target="_blank"
            to="https://github.com/Logical-Byte/open-endfield-assistant"
            variant="ghost"
          />
        </template>
      </UHeader>

      <UMain>
        <RouterView />
      </UMain>

      <USeparator icon="simple-icons:vuedotjs" />

      <UFooter>
        <template #left>
          <p class="text-sm text-muted">Built with Nuxt UI • © {{ new Date().getFullYear() }}</p>
        </template>

        <template #right>
          <UButton
            aria-label="GitHub"
            color="neutral"
            icon="simple-icons:github"
            target="_blank"
            to="https://github.com/nuxt-ui-templates/starter-vue"
            variant="ghost"
          />
        </template>
      </UFooter>
    </UApp>
  </Suspense>
</template>
