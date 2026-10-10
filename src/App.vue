<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import { initAutomationState } from '@/features/automation/state';
import { initArchiveCatalog } from '@/features/gameData/archiveCatalog';
import {
  settingsSaveError,
  initOeaSettings,
  markSettingsUnsupported,
  retrySettingsSave,
  settingsState,
} from '@/features/settings/settings';
import { initLogState } from '@/features/log/logState';
import { initScannedItems } from '@/features/archiveScan/scannedItems';
import { initUpdateState } from '@/features/update/update';
import { initUiScale } from '@/features/appearance/uiScale';
import { isTauri } from '@tauri-apps/api/core';
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed, watch } from 'vue';
import { zh_cn } from '@nuxt/ui/locale';
import { useAppI18n } from '@/shared/i18n';
import { bindTranslatedToasts, useTranslatedToast } from '@/shared/i18n/toast';
import { useRoute, useRouter } from 'vue-router';

const { t, locale } = useAppI18n();
const toast = useTranslatedToast();
bindTranslatedToasts();
const componentLocale = zh_cn;
const route = useRoute();
const router = useRouter();
// 扫描提示也能触发设置保存，失败通知放在应用层以覆盖设置页以外的操作。
watch(settingsSaveError, (error) => {
  if (error) {
    toast.add({ color: 'error' }, () => ({
      title: t('settings.save.failed.title'),
      description: t('settings.save.failed.description'),
      actions: [{ label: t('common.retrySave'), onClick: retrySettingsSave }],
    }));
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
  toast.add({ color: 'error' }, () => ({
    title: t('settings.initialize.failed.title'),
    description: t('settings.initialize.failed.description'),
    actions: [{ label: t('settings.initialize.open'), onClick: () => router.push('/settings') }],
  }));
});

const colorMode = useColorMode();
const themeColor = computed(() => (colorMode.value === 'dark' ? '#18181b' : '#ffffff'));
const { style, link } = useTheme();

useHead({
  title: computed(() => t('application.title')),
  htmlAttrs: { lang: locale },
  style,
  link,
  meta: [
    { name: 'theme-color', content: themeColor },
    { name: 'description', content: computed(() => t('application.title')) },
    { property: 'og:title', content: computed(() => t('application.title')) },
    { property: 'og:description', content: computed(() => t('application.title')) },
  ],
});

async function initApp(): Promise<void> {
  if (!isTauri()) {
    markSettingsUnsupported();
    return;
  }
  await initAutomationState();
  await initArchiveCatalog();
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
    <UApp :locale="componentLocale" :toaster="{ position: 'bottom-right', max: 8, expand: true }">
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
