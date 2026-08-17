<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import { initAppStatus } from '@/utils/app/appStatus';
import { initAppVersion } from '@/utils/app/appVersion';
import { initArchiveAcquisitionContract } from '@/utils/app/archiveAcquisitionContract';
import { initOeaConfig } from '@/utils/app/config';
import { initLogState } from '@/utils/app/logState';
import { initPrtsData } from '@/utils/app/prtsData';
import { initScanResults } from '@/utils/app/scanResults';
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed } from 'vue';

const colorMode = useColorMode();
const themeColor = computed(() => (colorMode.value === 'dark' ? '#18181b' : '#ffffff'));
const { style, link } = useTheme();

useHead({
  style,
  link,
  meta: [{ name: 'theme-color', content: themeColor }],
});

initAppStatus();
initAppVersion();
initLogState();
await initOeaConfig();
initArchiveAcquisitionContract();
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
        <InstallUpdateModal />
      </div>
    </UApp>
  </Suspense>
</template>
