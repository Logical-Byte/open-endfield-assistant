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
      <AppHeader />

      <UMain>
        <RouterView />
      </UMain>

      <!-- <AppFooter /> -->

      <AppImagePreview />
    </UApp>
  </Suspense>
</template>
