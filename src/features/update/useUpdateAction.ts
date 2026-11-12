import { computed, type ComputedRef } from 'vue';
import type { DownloadProgress, UpdateInfo } from './types/update';
import type { InstallConfirmation } from './update';
import { appVersion } from '@/app/build';
import { automationStatus } from '@/features/automation/state';
import { settingsState } from '@/features/settings/settings';
import { t, n } from '@/shared/i18n';
import { formatBackendError } from '@/shared/errors';
import { updateCapabilities } from './variantPolicy';
import {
  availableUpdate,
  pendingUpdate,
  downloadState,
  updateCheckState,
  effectiveOperation,
  localPackageSelected,
  installConfirmation,
  checkUpdate,
  startDownload,
  requestInstallConfirmation,
  confirmInstall,
  cancelInstallConfirmation,
  selectLocalUpdate,
  updateOperationBusy,
} from './update';

/** 所有更新入口共享动作与展示规则；调用者只决定按钮布局。 */
export function useUpdateAction(): {
  phase: ComputedRef<string>;
  release: ComputedRef<UpdateInfo | null>;
  version: ComputedRef<string>;
  progress: ComputedRef<DownloadProgress | null>;
  icon: ComputedRef<string>;
  spinning: ComputedRef<boolean>;
  label: ComputedRef<string>;
  tooltip: ComputedRef<string>;
  disabledReason: ComputedRef<string>;
  confirmation: ComputedRef<InstallConfirmation | null>;
  activate: () => Promise<void>;
  confirm: () => Promise<void>;
  cancel: () => Promise<void>;
  local: {
    disabled: ComputedRef<boolean>;
    busy: ComputedRef<boolean>;
    select: () => Promise<void>;
  };
} {
  const release = computed(() => pendingUpdate.value ?? availableUpdate.value);
  const version = computed(() => {
    const name = release.value?.versionName ?? '';
    return name.startsWith('v') ? name : `v${name}`;
  });
  const phase = computed(() => {
    if (effectiveOperation.value !== 'idle') return effectiveOperation.value;
    if (pendingUpdate.value) return 'ready';
    if (downloadState.value.status === 'failed') return 'downloadFailed';
    if (availableUpdate.value) return 'available';
    return 'idle';
  });
  const progress = computed(() =>
    downloadState.value.status === 'downloading' ? downloadState.value.progress : null,
  );
  const icon = computed(() =>
    phase.value === 'ready' || phase.value === 'installing'
      ? 'i-lucide-rotate-cw'
      : ['available', 'downloadFailed', 'downloading'].includes(phase.value)
        ? 'i-lucide-download'
        : 'i-lucide-refresh-cw',
  );
  const spinning = computed(() => phase.value === 'checking' || phase.value === 'installing');
  const disabledReason = computed(() => {
    if (!updateCapabilities.onlineUpdate) return t('settings.about.devUnavailable');
    if (settingsState.value.status === 'initializing') return t('settings.about.initializing');
    if (settingsState.value.status === 'unavailable') return t('settings.unavailable');
    if (effectiveOperation.value !== 'idle' || localPackageSelected.value)
      return t('settings.about.updateBusy');
    if (pendingUpdate.value && automationStatus.value.state !== 'idle')
      return t('update.install.afterTask');
    return '';
  });
  function bytes(value: number): string {
    const units = ['B', 'KB', 'MB', 'GB'];
    const index = value > 0 ? Math.min(3, Math.floor(Math.log(value) / Math.log(1024))) : 0;
    return `${n(value / 1024 ** index, { maximumFractionDigits: index === 0 ? 0 : 1 })}${units[index]}`;
  }
  const label = computed(() => {
    if (progress.value)
      return progress.value.totalSize > 0
        ? `${bytes(progress.value.downloadedSize)} / ${bytes(progress.value.totalSize)}`
        : bytes(progress.value.downloadedSize);
    if (phase.value === 'ready') return t('update.action.restart');
    if (phase.value === 'checking') return t('update.check.checking');
    if (phase.value === 'installing') return t('update.install.installing');
    if (phase.value === 'downloadFailed') return t('settings.about.retryDownload');
    if (phase.value === 'available')
      return t('update.action.published', { version: version.value });
    return t('update.check.action');
  });
  const tooltip = computed(() => {
    if (phase.value === 'ready')
      return disabledReason.value || t('update.action.readyTooltip', { version: version.value });
    if (progress.value)
      return t('update.action.progressTooltip', {
        version: version.value,
        percent: n(progress.value.progress / 100, 'percent'),
      });
    if (phase.value === 'available')
      return t('update.action.availableTooltip', {
        current: `v${appVersion}`,
        version: version.value,
      });
    if (downloadState.value.status === 'failed')
      return formatBackendError(downloadState.value.error);
    if (updateCheckState.value.status === 'error')
      return formatBackendError(updateCheckState.value.error);
    return disabledReason.value || label.value;
  });
  async function activate(): Promise<void> {
    if (disabledReason.value) return;
    if (pendingUpdate.value) requestInstallConfirmation();
    else if (availableUpdate.value) await startDownload('manual');
    else await checkUpdate('manual');
  }
  return {
    phase,
    release,
    version,
    progress,
    icon,
    spinning,
    label,
    tooltip,
    disabledReason,
    confirmation: computed(() => installConfirmation.value),
    activate,
    confirm: confirmInstall,
    cancel: cancelInstallConfirmation,
    local: {
      disabled: computed(
        () => updateOperationBusy.value || automationStatus.value.state !== 'idle',
      ),
      busy: computed(() => effectiveOperation.value === 'installing'),
      select: selectLocalUpdate,
    },
  };
}
