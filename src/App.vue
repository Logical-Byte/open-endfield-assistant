<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import { initAutomationState } from '@/features/automation/state';
import { initArchiveContract } from '@/features/gameData/archiveContract';
import {
  settingsSaveError,
  initOeaSettings,
  markSettingsUnsupported,
  retrySettingsSave,
  settingsState,
} from '@/features/settings/settings';
import { initLogState } from '@/features/log/logState';
import { initPrtsData } from '@/features/gameData/prtsData';
import { initScannedItems } from '@/features/archiveScan/scannedItems';
import { initUpdateState } from '@/features/update/update';
import { initUiScale } from '@/features/appearance/uiScale';
import { isTauri } from '@tauri-apps/api/core';
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const toast = useToast();
const route = useRoute();
const router = useRouter();
// 扫描提示也能触发设置保存，失败通知放在应用层以覆盖设置页以外的操作。
watch(settingsSaveError, (error) => {
  if (error) {
    toast.add({
      title: '设置未保存',
      description: '已保留当前编辑，应用仍使用最近一次成功保存的设置。',
      color: 'error',
      actions: [{ label: '重试保存', onClick: retrySettingsSave }],
    });
  }
});

let settingsInitializeErrorNotified = false;
watch([settingsState, () => route.path], ([state, path]) => {
  const initializeFailed =
    state.status === 'unavailable' && state.reason.type === 'initialize-error';
  if (!initializeFailed) {
    settingsInitializeErrorNotified = false;
    return;
  }
  if (path === '/settings' || settingsInitializeErrorNotified) return;

  settingsInitializeErrorNotified = true;
  toast.add({
    title: '设置初始化失败',
    description: '自动更新和扫描提示暂时不会使用用户设置。',
    color: 'error',
    actions: [{ label: '前往设置', onClick: () => router.push('/settings') }],
  });
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
  if (!isTauri()) {
    markSettingsUnsupported();
    return;
  }
  await initAutomationState();
  await initPrtsData();
  await initArchiveContract();
  await initLogState();
  await initOeaSettings();
  await initScannedItems();
  await initUiScale();
  await initUpdateState();
}

void initApp();
</script>

<template>
  <Suspense>
    <UApp :toaster="{ position: 'bottom-right', max: 8, expand: true }">
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
