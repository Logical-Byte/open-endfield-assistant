<script setup lang="ts">
import { AppVariant } from '@/app/build';
import { appPresentation } from '@/app/presentation';
import { automationStatus } from '@/features/automation/state';
import { settingsState } from '@/features/settings/settings';
import { UpdateInstallStatus, type UpdateInfo } from '@/features/update/types/update';
import {
  checkUpdate,
  downloadState,
  installStatus,
  pendingUpdate,
  retryInstall,
  startDownload,
  startInstall,
  updateCheckState,
  updateOperation,
  updateOperationBusy,
} from '@/features/update/update';
import { updateCapabilities } from '@/features/update/variantPolicy';
import { useAppI18n } from '@/shared/i18n';
import { useTranslatedToast, type ToastText } from '@/shared/i18n/toast';
import { computed } from 'vue';
import icon from '../../../resources/icons/icon.png';

const { t, locale, n } = useAppI18n();
const currentVersion = computed<string>((): string =>
  appPresentation.variant === AppVariant.Dev
    ? t('settings.about.dev')
    : `v${appPresentation.version}`,
);
interface AboutLink {
  label: string;
  icon: string;
  to: string;
  color: 'primary' | 'neutral';
  variant: 'solid' | 'outline';
}

const links = computed<AboutLink[]>((): AboutLink[] => [
  {
    label: t('settings.about.website'),
    icon: 'i-lucide-globe',
    to: 'https://ef.yituliu.cn/resources/oea',
    color: 'primary',
    variant: 'solid',
  },
  {
    label: 'GitHub',
    icon: 'i-simple-icons:github',
    to: 'https://github.com/Logical-Byte/open-endfield-assistant',
    color: 'neutral',
    variant: 'solid',
  },
  {
    label: t('settings.about.community'),
    icon: 'i-simple-icons:qq',
    to: 'https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms',
    color: 'primary',
    variant: 'solid',
  },
]);

// 使用共享更新投影，进入设置页不另行检查，也不缓存一份页面专属结果。
const newRelease = computed<UpdateInfo | null>((): UpdateInfo | null => {
  if (!updateCapabilities.onlineUpdate) return null;
  if (pendingUpdate.value) return pendingUpdate.value;
  return updateCheckState.value.status === 'available' ? updateCheckState.value.update : null;
});
const releaseVersion = computed<string>((): string => {
  const version = newRelease.value?.versionName ?? '';
  return version.startsWith('v') ? version : `v${version}`;
});
const releaseUrl = computed<string>(
  (): string =>
    `https://github.com/Logical-Byte/open-endfield-assistant/releases/tag/${encodeURIComponent(newRelease.value?.versionName ?? '')}`,
);
const lastCheckedText = computed<string>((): string => {
  const checkedAt = updateCheckState.value.lastCheckedAt;
  const time =
    checkedAt === null
      ? t('settings.about.notChecked')
      : new Intl.DateTimeFormat(locale.value, {
          hour: '2-digit',
          minute: '2-digit',
          hour12: false,
        }).format(checkedAt);
  return t('settings.about.lastChecked', { time });
});

const checking = computed<boolean>((): boolean => updateCheckState.value.status === 'checking');
const checkDisabledReason = computed<string>((): string => {
  if (!updateCapabilities.onlineUpdate) return t('settings.about.devUnavailable');
  if (updateOperationBusy.value) return t('settings.about.updateBusy');
  if (settingsState.value.status === 'initializing') return t('settings.about.initializing');
  if (
    settingsState.value.status === 'unavailable' &&
    settingsState.value.reason.type === 'unsupported'
  ) {
    return t('settings.unsupported.label');
  }
  return '';
});
const updateDisabledReason = computed<string>((): string => {
  if (updateOperation.value !== 'idle' || checking.value) return t('settings.about.updateBusy');
  if (downloadState.value.status === 'downloading' || downloadState.value.status === 'cancelling') {
    return t('settings.about.updateBusy');
  }
  if (pendingUpdate.value && automationStatus.value.state !== 'idle') {
    return t('update.install.taskRunning');
  }
  return '';
});
const updateLoading = computed<boolean>(
  (): boolean =>
    downloadState.value.status === 'downloading' ||
    downloadState.value.status === 'cancelling' ||
    installStatus.value === UpdateInstallStatus.Installing,
);
const updateLabel = computed<string>((): string => {
  if (installStatus.value === UpdateInstallStatus.Installing) return t('update.install.installing');
  if (downloadState.value.status === 'cancelling') return t('update.download.cancelling');
  if (downloadState.value.status === 'downloading') {
    return t('settings.about.downloading', {
      percent: n(downloadState.value.progress.progress / 100, 'percent'),
    });
  }
  if (pendingUpdate.value) return t('settings.about.restart', { version: releaseVersion.value });
  if (installStatus.value === UpdateInstallStatus.Failed) return t('update.install.redownload');
  if (downloadState.value.status === 'failed') return t('settings.about.retryDownload');
  return t('settings.about.update', { version: releaseVersion.value });
});

async function manualCheckUpdate(): Promise<void> {
  await checkUpdate();
  if (updateCheckState.value.status === 'upToDate') {
    useTranslatedToast().add(
      { icon: 'i-lucide-check-circle', color: 'success' },
      (): ToastText => ({
        title: t('update.check.upToDate'),
        description: currentVersion.value,
      }),
    );
  }
}

async function updateApplication(): Promise<void> {
  if (pendingUpdate.value) {
    await startInstall();
  } else if (installStatus.value === UpdateInstallStatus.Failed) {
    await retryInstall();
  } else {
    await startDownload();
  }
}
</script>

<template>
  <SettingsCard
    id="about"
    class="scroll-mt-8"
    icon="i-lucide-info"
    :title="t('settings.about.title')"
  >
    <template #body>
      <div class="grid items-center gap-5 sm:grid-cols-[max-content_minmax(0,1fr)]">
        <div class="flex items-center gap-3">
          <img alt="OEA" class="size-12 shrink-0" :src="icon" />
          <div class="min-w-0">
            <h2 class="text-lg font-semibold text-highlighted">OEA</h2>
            <p class="mt-1 text-sm text-muted">{{ currentVersion }}</p>
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-2 sm:justify-end">
          <UButton
            v-for="link in links"
            :key="link.to"
            :color="link.color"
            :icon="link.icon"
            :label="link.label"
            rel="noopener noreferrer"
            target="_blank"
            :to="link.to"
            :variant="link.variant"
          />
          <UTooltip :disabled="!checkDisabledReason" :text="checkDisabledReason">
            <span class="inline-flex" :tabindex="checkDisabledReason ? 0 : undefined">
              <UButton
                color="neutral"
                :disabled="Boolean(checkDisabledReason)"
                icon="i-lucide-refresh-cw"
                :label="checking ? t('update.check.checking') : t('update.check.action')"
                :loading="checking"
                variant="outline"
                @click="manualCheckUpdate"
              />
            </span>
          </UTooltip>
        </div>

        <div
          v-if="newRelease"
          class="flex flex-col items-start justify-between gap-3 rounded-lg border border-default bg-elevated p-4 sm:col-span-2 sm:flex-row sm:items-center sm:gap-6"
        >
          <div>
            <p class="text-xs text-muted">{{ t('update.check.available') }}</p>
            <p class="mt-1 text-[26px] leading-8 font-bold tracking-[-0.7px] text-primary">
              {{ releaseVersion }}
            </p>
            <p class="mt-1 text-xs text-muted">{{ lastCheckedText }}</p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <UButton
              color="neutral"
              :label="t('settings.about.releaseNotes')"
              rel="noopener noreferrer"
              target="_blank"
              :to="releaseUrl"
              trailing-icon="i-lucide-external-link"
              variant="outline"
            />
            <UTooltip :disabled="!updateDisabledReason" :text="updateDisabledReason">
              <span class="inline-flex" :tabindex="updateDisabledReason ? 0 : undefined">
                <UButton
                  :disabled="Boolean(updateDisabledReason)"
                  icon="i-lucide-download"
                  :label="updateLabel"
                  :loading="updateLoading"
                  @click="updateApplication"
                />
              </span>
            </UTooltip>
          </div>
        </div>
      </div>
      <section
        v-if="appPresentation.variant === AppVariant.Dev"
        class="mt-6 border-t border-default pt-5"
      >
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium text-toned">
          <UIcon class="size-4" name="i-lucide-info" />
          {{ t('settings.about.devRestrictions.title') }}
        </h3>
        <ul class="list-disc space-y-2 pl-5 text-sm text-muted">
          <li>{{ t('settings.about.devRestrictions.updates') }}</li>
          <li>{{ t('settings.about.devRestrictions.automaticUpdates') }}</li>
          <li>{{ t('settings.about.devRestrictions.localInstall') }}</li>
        </ul>
      </section>
    </template>
  </SettingsCard>
</template>
