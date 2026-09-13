import { oeaVersion } from '@/main';
import {
  DownloadProgress,
  DownloadState,
  UpdateAvailability,
  UpdateCheckState,
  UpdateCompleteInfo,
  UpdateInfo,
  UpdateInstallStage,
  UpdateInstallStageEvent,
  UpdateInstallStatus,
  UpdateOperation,
  UpdateStatus,
} from '@/types/update';
import { appStatus } from '@/utils/app/appStatus';
import { oeaConfig } from '@/utils/app/config';
import { logError, logInfo, logWarn, onAppStatus } from '@/utils/tauri';
import { updatePopoverOpen } from '@/utils/uiState';
import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { computed, ref, shallowRef } from 'vue';

const EMPTY_DOWNLOAD_PROGRESS: DownloadProgress = {
  downloadedSize: 0,
  totalSize: 0,
  speed: 0,
  progress: 0,
};

const INITIAL_UPDATE_STATUS: UpdateStatus = {
  operation: 'idle',
  availableUpdate: null,
  pendingUpdate: null,
};

/** 后端更新业务状态的只读 WebView 投影。 */
const updateStatus = shallowRef<UpdateStatus>(INITIAL_UPDATE_STATUS);
export const availableUpdate = computed<UpdateInfo | null>(
  () => updateStatus.value.availableUpdate,
);
export const pendingUpdate = computed<UpdateInfo | null>(() => updateStatus.value.pendingUpdate);
export const updateOperation = computed<UpdateOperation>(() => updateStatus.value.operation);

/** IPC 被调用后、后端快照赶上前的即时 loading 投影。 */
const requestedOperation = ref<UpdateOperation | null>(null);
const effectiveOperation = computed<UpdateOperation>(
  () => requestedOperation.value ?? updateOperation.value,
);

/** 当前 WebView 生命周期内的检查展示状态。 */
const lastCheckedAt = ref<number | null>(null);
const checkError = ref<Error | null>(null);
export const updateCheckState = computed<UpdateCheckState>(() => {
  if (effectiveOperation.value === 'checking') {
    return { status: 'checking', lastCheckedAt: lastCheckedAt.value };
  }
  if (checkError.value) {
    return { status: 'error', lastCheckedAt: lastCheckedAt.value, error: checkError.value };
  }
  if (availableUpdate.value) {
    return {
      status: 'available',
      lastCheckedAt: lastCheckedAt.value,
      update: availableUpdate.value,
    };
  }
  if (lastCheckedAt.value !== null) {
    return { status: 'upToDate', lastCheckedAt: lastCheckedAt.value };
  }
  return { status: 'unknown', lastCheckedAt: null };
});

/** 下载进度、取消请求和错误都是本次 WebView 调用的展示状态。 */
const downloadProgress = ref<DownloadProgress>(EMPTY_DOWNLOAD_PROGRESS);
const downloadCancelling = ref<boolean>(false);
const downloadFailed = ref<boolean>(false);
export const downloadState = computed<DownloadState>(() => {
  if (effectiveOperation.value === 'downloading') {
    return downloadCancelling.value
      ? { status: 'cancelling', progress: downloadProgress.value }
      : { status: 'downloading', progress: downloadProgress.value };
  }
  if (pendingUpdate.value) {
    return { status: 'completed', update: pendingUpdate.value };
  }
  if (downloadFailed.value) {
    return { status: 'failed' };
  }
  return { status: 'idle' };
});

/** 安装阶段状态。 */
export const installStatus = ref<UpdateInstallStatus>(UpdateInstallStatus.Idle);
/** 安装流程当前阶段（驱动弹窗进度文案）。 */
export const installStage = ref<UpdateInstallStage | null>(null);
/** 安装失败原因。 */
export const installError = ref<string | null>(null);
/** 重启后展示的「更新完成」信息。 */
export const justUpdatedInfo = ref<UpdateCompleteInfo | null>(null);
/** 安装弹窗是否打开。 */
export const showInstallModal = ref<boolean>(false);

/** 后端业务状态或当前 IPC 请求占用更新入口时，不允许发起其他更新操作。 */
export const updateOperationBusy = computed<boolean>(
  () =>
    requestedOperation.value !== null ||
    updateOperation.value !== 'idle' ||
    pendingUpdate.value !== null,
);

/** 安装阶段 → 用户可读文案。 */
const INSTALL_STAGE_LABELS: Record<UpdateInstallStage, string> = {
  [UpdateInstallStage.Preparing]: '准备更新文件',
  [UpdateInstallStage.Extracting]: '解压更新包',
  [UpdateInstallStage.ApplyingIncremental]: '应用增量更新',
  [UpdateInstallStage.ApplyingFull]: '应用全量更新',
  [UpdateInstallStage.CleaningUp]: '清理临时文件',
};

/** 安装阶段文案（供弹窗展示）。 */
export function installStageLabel(stage: UpdateInstallStage): string {
  return INSTALL_STAGE_LABELS[stage];
}

/** 用一次 IPC 替换完整后端状态投影。 */
export async function refreshUpdateStatus(): Promise<void> {
  try {
    updateStatus.value = await invoke<UpdateStatus>('get_update_status');
  } catch (error) {
    logWarn(`读取更新状态失败: ${String(error)}`);
  }
}

/** 执行一次检查更新（启动自动检查与设置页手动检查共用）。 */
export async function checkUpdate(): Promise<void> {
  if (updateOperationBusy.value) {
    return;
  }

  requestedOperation.value = 'checking';
  checkError.value = null;
  let shouldAutoDownload = false;
  try {
    const availability = await invoke<UpdateAvailability>('check_update');
    lastCheckedAt.value = Date.now();
    if (availability.status === 'available') {
      logWarn(
        `检查更新：有新版本可用，当前 v${oeaVersion}，最新 ${availability.update.versionName}`,
      );
      updatePopoverOpen.value = true;
      shouldAutoDownload = oeaConfig.value.autoDownloadUpdates;
    } else {
      logInfo(`检查更新：已是最新版本 v${oeaVersion}`);
    }
  } catch (error) {
    checkError.value = error instanceof Error ? error : new Error(String(error));
    updatePopoverOpen.value = true;
    logError(`检查更新失败: ${checkError.value.message}`);
  } finally {
    await refreshUpdateStatus();
    requestedOperation.value = null;
  }

  if (shouldAutoDownload) {
    void startDownload();
  }
}

/** 开始下载后端缓存的更新，并通过本次调用独享的 Channel 接收进度。 */
export async function startDownload(): Promise<void> {
  if (
    effectiveOperation.value !== 'idle' ||
    availableUpdate.value === null ||
    pendingUpdate.value !== null
  ) {
    return;
  }

  requestedOperation.value = 'downloading';
  downloadProgress.value = EMPTY_DOWNLOAD_PROGRESS;
  downloadCancelling.value = false;
  downloadFailed.value = false;
  updatePopoverOpen.value = true;

  const onProgress = new Channel<DownloadProgress>((progress) => {
    downloadProgress.value = progress;
  });
  let completed = false;
  try {
    const update = await invoke<UpdateInfo>('download_update', { onProgress });
    completed = true;
    logInfo(`更新下载完成: ${update.versionName}`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message === '下载已取消') {
      logInfo('下载已被用户取消');
    } else {
      downloadFailed.value = true;
      handleDownloadFailure(error, '下载失败');
    }
  } finally {
    await refreshUpdateStatus();
    requestedOperation.value = null;
    downloadCancelling.value = false;
  }

  if (completed) {
    void tryAutoInstall();
  }
}

/** 取消当前下载（Rust 置取消标志，临时文件由守卫清理）。 */
export async function cancelDownload(): Promise<void> {
  if (effectiveOperation.value !== 'downloading' || downloadCancelling.value) {
    return;
  }
  downloadCancelling.value = true;
  try {
    await invoke('cancel_download');
  } catch (error) {
    logWarn(`取消下载失败: ${error instanceof Error ? error.message : String(error)}`);
  } finally {
    await refreshUpdateStatus();
  }
}

/**
 * 启动时先消费 Rust transaction 的完成结果，再读取一次当前进程的更新状态。
 * 待安装更新只存在于当前 Rust 进程，不从 WebView 存储恢复。
 */
export async function initUpdateState(): Promise<void> {
  const startupUpdateResult = await consumeStartupUpdateResult();
  await refreshUpdateStatus();
  if (startupUpdateResult === 'completed') {
    installStatus.value = UpdateInstallStatus.Completed;
    installError.value = null;
    installStage.value = null;
    justUpdatedInfo.value = { timestamp: Date.now() };
    showInstallModal.value = true;
  }

  await onAppStatus((status) => {
    if (!status.running) {
      void tryAutoInstall();
    }
  });

  if (pendingUpdate.value) {
    if (oeaConfig.value.autoInstallUpdates) {
      void tryAutoInstall();
    } else {
      updatePopoverOpen.value = true;
    }
  } else {
    await checkUpdate();
  }
}

/** Rust 启动恢复的最小返回值：完成一次资源事务，或没有已完成的事务。 */
type StartupUpdateResult = 'completed' | null;

/** 查询并消费本次启动是否完成了一个更新事务。 */
async function consumeStartupUpdateResult(): Promise<StartupUpdateResult> {
  try {
    return await invoke<StartupUpdateResult>('consume_startup_update_result');
  } catch (error) {
    logWarn(`读取启动更新结果失败: ${String(error)}`);
    return null;
  }
}

/** 满足条件时自动开始安装：存在待安装更新、开启自动安装且扫描空闲。 */
export async function tryAutoInstall(): Promise<void> {
  if (
    pendingUpdate.value === null ||
    effectiveOperation.value !== 'idle' ||
    installStatus.value !== UpdateInstallStatus.Idle ||
    !oeaConfig.value.autoInstallUpdates ||
    appStatus.value.running
  ) {
    return;
  }
  await startInstall();
}

/** 安装启动结果：命令已接受、流程被条件阻止，或安装失败。 */
export type InstallStartResult = 'started' | 'skipped' | 'failed';

/** 开始安装（自动触发与手动「立即安装」共用；扫描任务运行中拒绝）。 */
export async function startInstall(): Promise<InstallStartResult> {
  if (pendingUpdate.value === null || effectiveOperation.value !== 'idle') {
    return 'skipped';
  }
  if (appStatus.value.running) {
    useToast().add({
      title: '扫描任务运行中',
      description: '扫描结束后将自动安装更新',
      icon: 'i-lucide-info',
      color: 'info',
    });
    return 'skipped';
  }

  requestedOperation.value = 'installing';
  beginInstallPresentation();
  let unlisten: (() => void) | null = null;
  try {
    unlisten = await listen<UpdateInstallStageEvent>('update-install-stage', (event) => {
      installStage.value = event.payload.stage;
    });
    await invoke('install_update');
    return 'started';
  } catch (error) {
    handleInstallFailure(error);
    return 'failed';
  } finally {
    unlisten?.();
    await refreshUpdateStatus();
    requestedOperation.value = null;
  }
}

/** 开发者本地包的高层 Rust 安装命令返回值。 */
export type DeveloperInstallResult = 'started' | 'cancelled' | 'failed';

/** 选择、暂存并安装开发者本地包，路径始终留在 Rust 内部。 */
export async function startDeveloperInstall(
  onStage: (stage: UpdateInstallStage) => void,
): Promise<DeveloperInstallResult> {
  if (updateOperationBusy.value) {
    return 'failed';
  }

  requestedOperation.value = 'installing';
  installError.value = null;
  let installationStarted = false;
  let unlisten: (() => void) | null = null;
  try {
    unlisten = await listen<UpdateInstallStageEvent>('update-install-stage', (event) => {
      if (!installationStarted) {
        installationStarted = true;
        beginInstallPresentation();
      }
      installStage.value = event.payload.stage;
      onStage(event.payload.stage);
    });
    const accepted = await invoke<boolean>('developer_install_update');
    return accepted ? 'started' : 'cancelled';
  } catch (error) {
    if (installationStarted) {
      handleInstallFailure(error);
    } else {
      installError.value = error instanceof Error ? error.message : String(error);
    }
    return 'failed';
  } finally {
    unlisten?.();
    await refreshUpdateStatus();
    requestedOperation.value = null;
  }
}

/** 安装失败后重新下载，绝不复用已消费的 ZIP 或 candidate。 */
export async function retryInstall(): Promise<void> {
  if (installStatus.value !== UpdateInstallStatus.Failed) {
    return;
  }
  installStatus.value = UpdateInstallStatus.Idle;
  installError.value = null;
  installStage.value = null;
  showInstallModal.value = false;
  downloadFailed.value = false;

  if (availableUpdate.value) {
    await startDownload();
  } else {
    await checkUpdate();
  }
}

/** 关闭安装弹窗（仅失败 / 完成展示可关闭；安装中不可关闭由弹窗控制）。 */
export function closeInstallModal(): void {
  showInstallModal.value = false;
  if (installStatus.value === UpdateInstallStatus.Failed) {
    installStatus.value = UpdateInstallStatus.Idle;
    installError.value = null;
    installStage.value = null;
  }
  if (justUpdatedInfo.value) {
    justUpdatedInfo.value = null;
    installStatus.value = UpdateInstallStatus.Idle;
  }
}

function beginInstallPresentation(): void {
  installStatus.value = UpdateInstallStatus.Installing;
  installError.value = null;
  installStage.value = null;
  showInstallModal.value = true;
  updatePopoverOpen.value = false;
}

function handleInstallFailure(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  installStatus.value = UpdateInstallStatus.Failed;
  installError.value = message;
  logError(`更新安装失败: ${message}`);
}

function handleDownloadFailure(error: unknown, fallbackTitle: string): void {
  const message = error instanceof Error ? error.message : String(error);
  logError(`${fallbackTitle}: ${message}`);
  useToast().add({
    title: fallbackTitle,
    description: message,
    icon: 'i-lucide-triangle-alert',
    color: 'error',
  });
}
