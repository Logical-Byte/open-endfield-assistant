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
      <div class="flex h-full flex-col">
        <AppHeader class="static z-auto backdrop-blur-none" />

        <UMain class="flex min-h-0 flex-1">
          <RouterView />
        </UMain>

        <!-- <AppFooter /> -->

        <AppImagePreview />
      </div>
    </UApp>
  </Suspense>
</template>
