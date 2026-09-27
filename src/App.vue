<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import { initAppStatus } from '@/utils/app/appStatus';
import { initArchiveContract } from '@/utils/app/archiveContract';
import { configSaveError, initOeaConfig, retrySettingsSave } from '@/utils/app/config';
import { initLogState } from '@/utils/app/logState';
import { initPrtsData } from '@/utils/app/prtsData';
import { initScanResults } from '@/utils/app/scanResults';
import { initUpdateState } from '@/utils/app/update';
import { initUiScale } from '@/utils/uiScale';
import { isTauri } from '@tauri-apps/api/core';
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed, watch } from 'vue';

const toast = useToast();
// 扫描提示也能触发设置保存，失败通知放在应用层以覆盖设置页以外的操作。
watch(configSaveError, (error) => {
  if (error) {
    toast.add({
      title: '设置未保存',
      description: '已保留当前编辑，应用仍使用最近一次成功保存的设置。',
      color: 'error',
      actions: [{ label: '重试保存', onClick: retrySettingsSave }],
    });
  }
});

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
    await initPrtsData();
    await initArchiveContract();
    await initLogState();
    await initOeaConfig();
    await initScanResults();
    await initUiScale();
    await initUpdateState();
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

        <UMain class="min-h-0 flex-1 overflow-y-auto">
          <RouterView />
        </UMain>

        <InstallUpdateModal />

        <!-- <AppFooter /> -->

        <AppImagePreview />
      </div>
    </UApp>
  </Suspense>
</template>
