import type {
  DownloadProgress,
  DownloadState,
  UpdateCheckState,
  UpdateCompleteInfo,
  UpdateInfo,
  UpdateInstallStage,
  UpdateOperation,
  UpdateStatus,
} from '@/features/update/types/update';
import { UpdateInstallStatus } from '@/features/update/types/update';
import { automationStatus } from '@/features/automation/state';
import { settingsState } from '@/features/settings/settings';
import type { DraftSettings } from '@/features/settings/settingsStore';
import { logDebug, logError, logWarn } from '@/features/log/ipc';
import { updatePopoverOpen } from '@/features/update/updatePopover';
import {
  getUpdateStatus,
  requestUpdateCheck,
  createDownloadProgressChannel,
  downloadUpdate,
  requestDownloadCancellation,
  takeStartupUpdateResult,
  onUpdateInstallStage,
  installUpdate,
  developerInstallUpdate,
  type StartupUpdateResult,
} from './ipc';
import { computed, ref, shallowRef, watch } from 'vue';

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

type LogWriter = (message: string) => Promise<void>;

/** Settings 初始化成功后，返回可供业务决策使用的最近一次持久化快照。 */
function currentEffectiveSettings(): Readonly<DraftSettings> | null {
  return settingsState.value.status === 'ready' ? settingsState.value.effective : null;
}

/** 日志 IPC 失败不能打断更新流程。浏览器控制台保留最后一层诊断信息。 */
function writeUpdateLog(write: LogWriter, message: string): void {
  void write(message).catch((error: unknown) => {
    console.error(`更新前端：写入后端日志失败: ${String(error)}`);
  });
}

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
  preparing: '准备更新文件',
  extracting: '解压更新包',
  applying_incremental: '应用增量更新',
  applying_full: '应用全量更新',
  cleaning_up: '清理临时文件',
};

/** 安装阶段文案（供弹窗展示）。 */
export function installStageLabel(stage: UpdateInstallStage): string {
  return INSTALL_STAGE_LABELS[stage];
}

/** 用一次 IPC 替换完整后端状态投影。 */
export async function refreshUpdateStatus(): Promise<void> {
  try {
    updateStatus.value = await getUpdateStatus();
  } catch (error) {
    writeUpdateLog(logWarn, `更新前端：读取后端更新状态失败: ${String(error)}`);
  }
}

/** 执行一次检查更新（启动自动检查与设置页手动检查共用）。 */
export async function checkUpdate(): Promise<void> {
  if (updateOperationBusy.value) {
    writeUpdateLog(
      logDebug,
      `更新前端：跳过检查请求，更新入口忙碌（requested=${requestedOperation.value ?? 'none'}, backend=${updateOperation.value}, pending=${pendingUpdate.value !== null}）`,
    );
    return;
  }

  requestedOperation.value = 'checking';
  checkError.value = null;
  let shouldAutoDownload = false;
  try {
    const availability = await requestUpdateCheck();
    lastCheckedAt.value = Date.now();
    if (availability.status === 'available') {
      const settings = currentEffectiveSettings();
      writeUpdateLog(
        logDebug,
        `更新前端：检测到可用更新，打开更新提示（autoDownload=${settings?.autoDownloadUpdates ?? 'n/a'}）`,
      );
      updatePopoverOpen.value = true;
      shouldAutoDownload = settings?.autoDownloadUpdates === true;
    }
  } catch (error) {
    checkError.value = error instanceof Error ? error : new Error(String(error));
    updatePopoverOpen.value = true;
    writeUpdateLog(
      logError,
      `更新前端：check_update 调用失败，显示检查错误: ${checkError.value.message}`,
    );
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
    writeUpdateLog(
      logDebug,
      `更新前端：跳过下载请求（effective=${effectiveOperation.value}, available=${availableUpdate.value !== null}, pending=${pendingUpdate.value !== null}）`,
    );
    return;
  }

  requestedOperation.value = 'downloading';
  downloadProgress.value = EMPTY_DOWNLOAD_PROGRESS;
  downloadCancelling.value = false;
  downloadFailed.value = false;
  updatePopoverOpen.value = true;

  const onProgress = createDownloadProgressChannel((progress) => {
    downloadProgress.value = progress;
  });
  let completed = false;
  try {
    const update = await downloadUpdate(onProgress);
    completed = true;
    writeUpdateLog(logDebug, `更新前端：download_update 调用完成（version=${update.versionName}）`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message === '下载已取消') {
      writeUpdateLog(logDebug, '更新前端：download_update 已确认取消');
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
    writeUpdateLog(
      logDebug,
      `更新前端：跳过取消下载请求（effective=${effectiveOperation.value}, cancelling=${downloadCancelling.value}）`,
    );
    return;
  }
  downloadCancelling.value = true;
  try {
    await requestDownloadCancellation();
  } catch (error) {
    writeUpdateLog(
      logWarn,
      `更新前端：cancel_download 调用失败: ${error instanceof Error ? error.message : String(error)}`,
    );
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
  writeUpdateLog(
    logDebug,
    `更新前端：初始化更新投影（startup=${startupUpdateResult}, backend=${updateOperation.value}, pending=${pendingUpdate.value !== null}）`,
  );
  if (startupUpdateResult === 'completed') {
    installStatus.value = UpdateInstallStatus.Completed;
    installError.value = null;
    installStage.value = null;
    justUpdatedInfo.value = { timestamp: Date.now() };
    showInstallModal.value = true;
  }

  watch(automationStatus, (status) => {
    if (status.state === 'idle') {
      void tryAutoInstall();
    }
  });

  if (pendingUpdate.value) {
    if (currentEffectiveSettings()?.autoInstallUpdates === true) {
      void tryAutoInstall();
    } else {
      updatePopoverOpen.value = true;
    }
  } else {
    await checkUpdate();
  }
}

/** 查询并消费本次启动是否完成了一个更新事务。 */
async function consumeStartupUpdateResult(): Promise<StartupUpdateResult> {
  try {
    return await takeStartupUpdateResult();
  } catch (error) {
    writeUpdateLog(logWarn, `更新前端：读取启动更新结果失败: ${String(error)}`);
    return 'no_transaction';
  }
}

/** 满足条件时自动开始安装：存在待安装更新、开启自动安装且扫描空闲。 */
export async function tryAutoInstall(): Promise<void> {
  const autoInstallEnabled = currentEffectiveSettings()?.autoInstallUpdates;
  if (
    pendingUpdate.value === null ||
    effectiveOperation.value !== 'idle' ||
    installStatus.value !== UpdateInstallStatus.Idle ||
    autoInstallEnabled !== true ||
    automationStatus.value.state !== 'idle'
  ) {
    writeUpdateLog(
      logDebug,
      `更新前端：跳过自动安装（pending=${pendingUpdate.value !== null}, effective=${effectiveOperation.value}, install=${installStatus.value}, enabled=${autoInstallEnabled ?? 'n/a'}, automating=${automationStatus.value.state !== 'idle'}）`,
    );
    return;
  }
  await startInstall();
}

/** 安装启动结果：命令已接受、流程被条件阻止，或安装失败。 */
export type InstallStartResult = 'started' | 'skipped' | 'failed';

/** 开始安装（自动触发与手动「立即安装」共用，扫描任务运行中拒绝）。 */
export async function startInstall(): Promise<InstallStartResult> {
  if (pendingUpdate.value === null || effectiveOperation.value !== 'idle') {
    writeUpdateLog(
      logDebug,
      `更新前端：跳过安装请求（pending=${pendingUpdate.value !== null}, effective=${effectiveOperation.value}）`,
    );
    return 'skipped';
  }
  if (automationStatus.value.state !== 'idle') {
    writeUpdateLog(logDebug, '更新前端：自动化任务运行中，安装请求留待任务结束后重试');
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
    unlisten = await onUpdateInstallStage((event) => {
      installStage.value = event.payload.stage;
    });
    await installUpdate();
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
    unlisten = await onUpdateInstallStage((event) => {
      if (!installationStarted) {
        installationStarted = true;
        beginInstallPresentation();
      }
      installStage.value = event.payload.stage;
      onStage(event.payload.stage);
    });
    const accepted = await developerInstallUpdate();
    writeUpdateLog(logDebug, `更新前端：developer_install_update 调用完成（accepted=${accepted}）`);
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

/** 关闭安装弹窗（仅失败 / 完成展示可关闭，安装中不可关闭由弹窗控制）。 */
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
  writeUpdateLog(logError, `更新前端：install_update 调用失败，显示安装错误: ${message}`);
}

function handleDownloadFailure(error: unknown, fallbackTitle: string): void {
  const message = error instanceof Error ? error.message : String(error);
  writeUpdateLog(logError, `更新前端：download_update 调用失败，显示下载错误: ${message}`);
  useToast().add({
    title: fallbackTitle,
    description: message,
    icon: 'i-lucide-triangle-alert',
    color: 'error',
  });
}
