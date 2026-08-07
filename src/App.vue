<script setup lang="ts">
import { initAppStatus } from '@/utils/app/appStatus';
import { initAppVersion } from '@/utils/app/appVersion';
import { initLogState } from '@/utils/app/logState';
import { initPrtsData } from '@/utils/app/prtsData';
import { initScanResults } from '@/utils/app/scanResults';
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed } from 'vue';

const colorMode = useColorMode();
const themeColor = computed(() => (colorMode.value === 'dark' ? '#18181b' : '#ffffff'));

useHead({
  meta: [{ name: 'theme-color', content: themeColor }],
});

initAppStatus();
initAppVersion();
initLogState();
initPrtsData();
initScanResults();
</script>

<template>
  <Suspense>
    <UApp>
      <div class="flex h-full flex-col">
        <TitleBar />
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
