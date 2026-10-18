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
import { appPresentation } from '@/app/presentation';
import { isTauri } from '@tauri-apps/api/core';
import { useHead } from '@unhead/vue';
import { useColorMode } from '@vueuse/core';
import { computed, watch } from 'vue';
import { en, zh_cn } from '@nuxt/ui/locale';
import { i18n, useAppI18n } from '@/shared/i18n';
import { bindTranslatedToasts, useTranslatedToast } from '@/shared/i18n/toast';
import { useRoute, useRouter } from 'vue-router';

const { t, locale } = useAppI18n();
const toast = useTranslatedToast();
bindTranslatedToasts();
watch(
  settingsState,
  (state) => {
    if (state.status === 'ready') void i18n.changeLanguage(state.effective.uiLocale);
  },
  { immediate: true },
);
const componentLocale = computed(() => (locale.value === 'zh-CN' ? zh_cn : en));
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
  title: computed(() => appPresentation.title()),
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

// 设计假设：用户的实际操作发生在 initApp 完成之后。
// 除非任务明确要求，否则实现与评审均不考虑用户操作和 initApp 初始化之间的竞争。
// 此假设仅限启动阶段。初始化完成后的设置修改、任务启停等正常交互仍需保证状态切换正确。
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
        <!-- 侧栏的拖拽手柄也是直接子元素，由中间列统一提供 8px 间距。 -->
        <UDashboardGroup
          class="relative! inset-auto! grid min-h-0 flex-1 grid-cols-[auto_8px_minmax(0,1fr)] p-2"
          :persistent="false"
          unit="px"
        >
          <AppSidebar />
          <main class="min-h-0 min-w-0 overflow-y-auto">
            <RouterView />
          </main>
        </UDashboardGroup>

        <InstallUpdateModal />

        <AppImagePreview />
      </div>
    </UApp>
  </Suspense>
</template>
