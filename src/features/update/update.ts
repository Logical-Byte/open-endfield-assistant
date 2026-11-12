import { computed, ref, shallowRef, watch } from 'vue';
import { whenever } from '@vueuse/core';
import { t } from '@/shared/i18n';
import { useTranslatedToast } from '@/shared/i18n/toast';
import { normalizeBackendError, formatBackendError, type ErrorFacts } from '@/shared/errors';
import { automationStatus } from '@/features/automation/state';
import type { DraftSettings } from '@/features/settings/settingsStore';
import { settingsState } from '@/features/settings/settings';
import { logError, logWarn } from '@/features/log/ipc';
import { updateCapabilities } from './variantPolicy';
import {
  UpdateInstallStatus,
  type DownloadProgress,
  type DownloadState,
  type UpdateCheckState,
  type UpdateOperation,
  type UpdateStatus,
} from './types/update';
import {
  getUpdateStatus,
  requestUpdateCheck,
  createDownloadProgressChannel,
  downloadUpdate,
  installUpdate,
  selectDeveloperUpdate,
  discardDeveloperUpdate,
  developerInstallUpdate,
} from './ipc';

export type UpdateTrigger = 'automatic' | 'manual';
export type InstallConfirmation = { kind: 'online'; version: string } | { kind: 'local' };
const updateStatus = shallowRef<UpdateStatus>({
  operation: 'idle',
  availableUpdate: null,
  pendingUpdate: null,
});
const requestedOperation = ref<UpdateOperation | null>(null);
export const effectiveOperation = computed(
  () => requestedOperation.value ?? updateStatus.value.operation,
);
export const availableUpdate = computed(() => updateStatus.value.availableUpdate);
export const pendingUpdate = computed(() => updateStatus.value.pendingUpdate);
export const updateOperation = computed(() => updateStatus.value.operation);
const lastCheckedAt = ref<number | null>(null);
const checkError = shallowRef<ErrorFacts | null>(null);
const downloadError = shallowRef<ErrorFacts | null>(null);
const downloadProgress = ref<DownloadProgress>({
  downloadedSize: 0,
  totalSize: 0,
  speed: 0,
  progress: 0,
});
// 来源属于本次下载，必须在后端发布 pending 状态前确定，避免手动下载被自动安装接走。
const downloadTrigger = ref<UpdateTrigger>('automatic');
export const localPackageSelected = ref(false);
export const installConfirmation = shallowRef<InstallConfirmation | null>(null);
export const installStatus = ref(UpdateInstallStatus.Idle);
export const showInstallModal = computed(
  () => installStatus.value === UpdateInstallStatus.Installing,
);
export const updateOperationBusy = computed(
  () =>
    effectiveOperation.value !== 'idle' ||
    pendingUpdate.value !== null ||
    localPackageSelected.value,
);
const taskIdle = computed(() => automationStatus.value.state === 'idle');
function effectiveSettings(): Readonly<DraftSettings> | null {
  return settingsState.value.status === 'ready' ? settingsState.value.effective : null;
}
export const updateCheckState = computed<UpdateCheckState>(() => {
  if (effectiveOperation.value === 'checking')
    return { status: 'checking', lastCheckedAt: lastCheckedAt.value };
  if (checkError.value)
    return { status: 'error', lastCheckedAt: lastCheckedAt.value, error: checkError.value };
  if (availableUpdate.value)
    return {
      status: 'available',
      lastCheckedAt: lastCheckedAt.value,
      update: availableUpdate.value,
    };
  if (lastCheckedAt.value !== null)
    return { status: 'upToDate', lastCheckedAt: lastCheckedAt.value };
  return { status: 'unknown', lastCheckedAt: null };
});
export const downloadState = computed<DownloadState>(() => {
  if (effectiveOperation.value === 'downloading')
    return { status: 'downloading', progress: downloadProgress.value };
  if (pendingUpdate.value) return { status: 'completed', update: pendingUpdate.value };
  if (downloadError.value) return { status: 'failed', error: downloadError.value };
  return { status: 'idle' };
});
function notifyFailure(error: unknown, stage: 'check' | 'download' | 'install'): ErrorFacts {
  const facts = normalizeBackendError(error, { operation: 'update', stage });
  const operation = { check: '检查更新', download: '下载更新', install: '安装更新' }[stage];
  void logError(`${operation}失败: ${formatBackendError(facts)}`).catch(console.error);
  useTranslatedToast().add({ color: 'error', icon: 'i-lucide-circle-alert' }, () => ({
    title: t(
      stage === 'check'
        ? 'update.check.failed'
        : stage === 'download'
          ? 'update.download.failed'
          : 'update.install.failed',
    ),
    description: formatBackendError(facts),
    actions: [{ label: t('common.viewLogs'), to: '/log' }],
  }));
  return facts;
}
export async function refreshUpdateStatus(): Promise<void> {
  try {
    updateStatus.value = await getUpdateStatus();
  } catch (error) {
    void logWarn(`读取更新状态失败: ${String(error)}`).catch(console.error);
  }
}
export async function checkUpdate(trigger: UpdateTrigger = 'manual'): Promise<void> {
  if (!updateCapabilities.onlineUpdate || updateOperationBusy.value) return;
  requestedOperation.value = 'checking';
  checkError.value = null;
  let autoDownload = false;
  try {
    const result = await requestUpdateCheck();
    lastCheckedAt.value = Date.now();
    autoDownload =
      result.status === 'available' && effectiveSettings()?.autoDownloadUpdates === true;
    if (result.status === 'upToDate' && trigger === 'manual') {
      useTranslatedToast().add({ color: 'success', icon: 'i-lucide-check-circle' }, () => ({
        title: t('update.check.upToDate'),
      }));
    }
  } catch (error) {
    checkError.value = notifyFailure(error, 'check');
  } finally {
    await refreshUpdateStatus();
    requestedOperation.value = null;
  }
  if (autoDownload) await startDownload(trigger);
}
export async function startDownload(trigger: UpdateTrigger = 'manual'): Promise<void> {
  if (
    effectiveOperation.value !== 'idle' ||
    !availableUpdate.value ||
    pendingUpdate.value ||
    localPackageSelected.value
  )
    return;
  downloadTrigger.value = trigger;
  requestedOperation.value = 'downloading';
  installStatus.value = UpdateInstallStatus.Idle;
  downloadError.value = null;
  downloadProgress.value = { downloadedSize: 0, totalSize: 0, speed: 0, progress: 0 };
  try {
    await downloadUpdate(
      createDownloadProgressChannel((progress) => {
        downloadProgress.value = progress;
      }),
    );
  } catch (error) {
    downloadError.value = notifyFailure(error, 'download');
  } finally {
    await refreshUpdateStatus();
    requestedOperation.value = null;
  }
}
export async function initUpdateState(): Promise<void> {
  if (!updateCapabilities.onlineUpdate && !updateCapabilities.localInstall) return;
  await refreshUpdateStatus();
  whenever(
    () =>
      pendingUpdate.value !== null &&
      downloadTrigger.value === 'automatic' &&
      effectiveSettings()?.autoInstallUpdates === true &&
      taskIdle.value &&
      effectiveOperation.value === 'idle' &&
      installStatus.value === UpdateInstallStatus.Idle,
    () => {
      void startInstall(false);
    },
    { immediate: true },
  );
  if (!pendingUpdate.value) {
    if (effectiveSettings()) await checkUpdate('automatic');
    else {
      const stop = watch(settingsState, (state) => {
        if (state.status === 'ready') {
          stop();
          void checkUpdate('automatic');
        }
      });
    }
  }
}
export function requestInstallConfirmation(): void {
  if (!taskIdle.value || effectiveOperation.value !== 'idle') return;
  if (localPackageSelected.value) installConfirmation.value = { kind: 'local' };
  else if (pendingUpdate.value)
    installConfirmation.value = { kind: 'online', version: pendingUpdate.value.versionName };
}
export async function cancelInstallConfirmation(): Promise<void> {
  const confirmation = installConfirmation.value;
  installConfirmation.value = null;
  if (confirmation?.kind === 'local') {
    requestedOperation.value = 'installing';
    try {
      await discardDeveloperUpdate();
    } catch (error) {
      notifyFailure(error, 'install');
    } finally {
      localPackageSelected.value = false;
      requestedOperation.value = null;
    }
  }
}
export async function confirmInstall(): Promise<void> {
  if (!installConfirmation.value || !taskIdle.value || effectiveOperation.value !== 'idle') return;
  const local = installConfirmation.value.kind === 'local';
  installConfirmation.value = null;
  await startInstall(true, local);
}
export async function startInstall(restart = false, local = false): Promise<void> {
  if (
    (!local && !pendingUpdate.value) ||
    (local && !localPackageSelected.value) ||
    effectiveOperation.value !== 'idle' ||
    !taskIdle.value
  )
    return;
  requestedOperation.value = 'installing';
  installStatus.value = UpdateInstallStatus.Installing;
  try {
    if (local) await developerInstallUpdate();
    else await installUpdate(restart);
  } catch (error) {
    installStatus.value = UpdateInstallStatus.Failed;
    notifyFailure(error, 'install');
  } finally {
    localPackageSelected.value = false;
    await refreshUpdateStatus();
    requestedOperation.value = null;
  }
}
export async function selectLocalUpdate(): Promise<void> {
  if (!updateCapabilities.localInstall || updateOperationBusy.value || !taskIdle.value) return;
  requestedOperation.value = 'installing';
  try {
    localPackageSelected.value = await selectDeveloperUpdate();
    if (localPackageSelected.value) installConfirmation.value = { kind: 'local' };
  } catch (error) {
    notifyFailure(error, 'install');
  } finally {
    requestedOperation.value = null;
  }
}
