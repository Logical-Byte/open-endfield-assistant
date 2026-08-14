<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import { initAppStatus } from '@/utils/app/appStatus';
import { initAppVersion } from '@/utils/app/appVersion';
import { initArchiveAcquisitionContract } from '@/utils/app/archiveAcquisitionContract';
import { initOeaConfig } from '@/utils/app/config';
import { initLogState } from '@/utils/app/logState';
import { initPrtsData } from '@/utils/app/prtsData';
import { initScanResults } from '@/utils/app/scanResults';
import { checkUpdate } from '@/utils/app/update';
import { initUiScale } from '@/utils/uiScale';
import { isTauri } from '@tauri-apps/api/core';
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

async function initApp(): Promise<void> {
  if (isTauri()) {
    await initAppStatus();
    await initAppVersion();
    await initPrtsData();
    await initArchiveAcquisitionContract();
    await initLogState();
    await initOeaConfig();
    await initScanResults();
    await initUiScale();
    await checkUpdate();
  }
}

void initApp();
</script>

<template>
  <Suspense>
    <UApp>
      <div class="flex h-full flex-col">
        <TitleBar />
        <AppHeader class="static z-auto backdrop-blur-none" />

        <UMain class="flex min-h-0 flex-1 overflow-y-auto">
          <RouterView />
        </UMain>

        <!-- <AppFooter /> -->

        <AppImagePreview />
      </div>
    </UApp>
  </Suspense>
</template>
