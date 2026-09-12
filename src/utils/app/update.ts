import { oeaVersion } from '@/main';
import {
  DownloadProgress,
  DownloadState,
  DownloadedUpdate,
  PendingUpdateInfo,
  UpdateAvailability,
  UpdateCheckState,
  UpdateCompleteInfo,
  UpdateInstallStageEvent,
  UpdateInstallStage,
  UpdateInstallStatus,
} from '@/types/update';
import { appStatus } from '@/utils/app/appStatus';
import { oeaConfig } from '@/utils/app/config';
import { logError, logInfo, logWarn, onAppStatus } from '@/utils/tauri';
import { updatePopoverOpen } from '@/utils/uiState';
import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ref } from 'vue';

const EMPTY_DOWNLOAD_PROGRESS: DownloadProgress = {
  downloadedSize: 0,
  totalSize: 0,
  speed: 0,
  progress: 0,
};

/** 检查更新状态（仅在当前 WebView 生命周期内保留成功时间）。 */
export const updateCheckState = ref<UpdateCheckState>({ status: 'unknown', lastCheckedAt: null });
/** 下载及其可安装结果的唯一状态。 */
export const downloadState = ref<DownloadState>({ status: 'idle' });
/** 安装阶段状态。 */
export const installStatus = ref<UpdateInstallStatus>(UpdateInstallStatus.Idle);
/** 安装流程当前阶段（驱动弹窗进度文案）。 */
export const installStage = ref<UpdateInstallStage | null>(null);
/** 安装失败原因（失败时可展示；重启失败时也复用此字段提示手动重启）。 */
export const installError = ref<string | null>(null);
/** 重启后展示的「更新完成」信息。 */
export const justUpdatedInfo = ref<UpdateCompleteInfo | null>(null);
/** 安装弹窗是否打开。 */
export const showInstallModal = ref<boolean>(false);

/** 安装互斥：同一时间只允许一个安装任务。 */
let isInstalling = false;

/** 待安装 / 更新完成信息的 localStorage key。 */
const PENDING_UPDATE_KEY = 'oea-pending-update';

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

/** 保存待安装更新信息（下载成功后调用，用于启动恢复与完成提示）。 */
function savePendingUpdateInfo(info: PendingUpdateInfo): void {
  try {
    localStorage.setItem(PENDING_UPDATE_KEY, JSON.stringify(info));
  } catch (error) {
    logWarn(`保存待安装更新信息失败: ${String(error)}`);
  }
}

/** 读取待安装更新信息，不访问磁盘。 */
function readPendingUpdateInfo(): PendingUpdateInfo | null {
  try {
    const raw = localStorage.getItem(PENDING_UPDATE_KEY);
    if (!raw) {
      return null;
    }
    const info = JSON.parse(raw) as PendingUpdateInfo;
    if (!info.downloadSavePath) {
      clearPendingUpdateInfo();
      return null;
    }
    return info;
  } catch (error) {
    logWarn(`读取待安装更新信息失败: ${String(error)}`);
    clearPendingUpdateInfo();
    return null;
  }
}

/** 读取待安装更新信息，并确认下载包仍然存在。 */
async function getPendingUpdateInfo(): Promise<PendingUpdateInfo | null> {
  const info = readPendingUpdateInfo();
  if (!info) {
    return null;
  }
  try {
    const exists = await invoke<boolean>('pending_package_exists', {
      packagePath: info.downloadSavePath,
    });
    if (!exists) {
      logWarn('待安装的更新包已被删除，清除待安装信息');
      clearPendingUpdateInfo();
      return null;
    }
    return info;
  } catch (error) {
    logWarn(`检查待安装更新包失败: ${String(error)}`);
    clearPendingUpdateInfo();
    return null;
  }
}

/** 清除待安装更新信息（安装完成 / 更新完成展示后）。 */
function clearPendingUpdateInfo(): void {
  try {
    localStorage.removeItem(PENDING_UPDATE_KEY);
  } catch {
    // localStorage 不可用时忽略
  }
}

/** 执行一次检查更新（启动自动检查与设置页手动检查共用）。 */
export async function checkUpdate(): Promise<void> {
  if (updateCheckState.value.status === 'checking') {
    return;
  }

  const lastCheckedAt = updateCheckState.value.lastCheckedAt;
  updateCheckState.value = { status: 'checking', lastCheckedAt };
  try {
    const availability = await invoke<UpdateAvailability>('check_update');
    const checkedAt = Date.now();
    if (availability.status === 'available') {
      updateCheckState.value = {
        status: 'available',
        lastCheckedAt: checkedAt,
        update: availability.update,
      };
      logWarn(
        `检查更新：有新版本可用，当前 v${oeaVersion}，最新 ${availability.update.versionName}`,
      );
      updatePopoverOpen.value = true;
      if (oeaConfig.value.autoDownloadUpdates && installStatus.value === UpdateInstallStatus.Idle) {
        void startDownload();
      }
    } else {
      logInfo(`检查更新：已是最新版本 v${oeaVersion}`);
      updateCheckState.value = { status: 'upToDate', lastCheckedAt: checkedAt };
    }
  } catch (error) {
    const errorInstance = error instanceof Error ? error : new Error(String(error));
    updateCheckState.value = {
      status: 'error',
      lastCheckedAt,
      error: errorInstance,
    };
    updatePopoverOpen.value = true;
    logError(`检查更新失败: ${errorInstance.message}`);
  }
}

/** 开始下载后端缓存的更新，并通过本次调用独享的 Channel 接收进度。 */
export async function startDownload(): Promise<void> {
  if (!['idle', 'failed'].includes(downloadState.value.status) || isInstalling) {
    return;
  }

  downloadState.value = { status: 'downloading', progress: EMPTY_DOWNLOAD_PROGRESS };
  updatePopoverOpen.value = true;

  const onProgress = new Channel<DownloadProgress>((progress) => {
    const status = downloadState.value.status;
    if (status === 'downloading' || status === 'cancelling') {
      downloadState.value = { status, progress };
    }
  });
  try {
    const update = await invoke<DownloadedUpdate>('download_update', { onProgress });
    downloadState.value = { status: 'completed', update };
    logInfo(`更新下载完成: ${update.downloadedPackagePath}`);
    savePendingUpdateInfo({
      previousVersion: oeaVersion,
      versionName: update.versionName,
      releaseNote: update.releaseNote,
      downloadSavePath: update.downloadedPackagePath,
      timestamp: Date.now(),
    });
    void tryAutoInstall();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message === '下载已取消') {
      logInfo('下载已被用户取消');
      downloadState.value = { status: 'idle' };
    } else {
      handleDownloadFailure(error, '下载失败');
    }
  }
}

/** 取消当前下载（Rust 置取消标志，临时文件由守卫清理）。 */
export async function cancelDownload(): Promise<void> {
  if (downloadState.value.status !== 'downloading') {
    return;
  }
  downloadState.value = { status: 'cancelling', progress: downloadState.value.progress };
  try {
    await invoke('cancel_download');
  } catch (error) {
    logWarn(`取消下载失败: ${error instanceof Error ? error.message : String(error)}`);
  }
}

/**
 * 启动时序：先让 Rust 完成一次尚未结束的 resources 事务，再读取待安装包。
 *
 * `consume_startup_update_result` 是只读的启动结果查询。它返回 `completed` 时，
 * helper 已经替换 exe，当前 v2 进程也已经提交 resources，zip 通常已经被删除。
 * pending metadata 可用于展示版本和更新日志，但缺失时仍应根据事务结果提示成功。
 */
export async function initUpdateState(): Promise<void> {
  const startupUpdateResult = await consumeStartupUpdateResult();
  if (startupUpdateResult === 'completed') {
    const pending = readPendingUpdateInfo();
    installStatus.value = UpdateInstallStatus.Completed;
    installError.value = null;
    installStage.value = null;
    if (pending) {
      justUpdatedInfo.value = {
        previousVersion: pending.previousVersion ?? '未知',
        newVersion: pending.versionName,
        releaseNote: pending.releaseNote,
        timestamp: Date.now(),
      };
    } else {
      // Rust 的启动结果是事务完成的事实来源。WebView 状态可能跨进程丢失，此时仍
      // 展示简化提示；以后可由 transaction 直接携带版本信息并统一两种展示。
      justUpdatedInfo.value = { timestamp: Date.now() };
      logWarn('启动更新事务已完成，但缺少待安装更新信息，显示简化完成提示');
    }
    showInstallModal.value = true;
    clearPendingUpdateInfo();
  } else {
    // helper 未能接管时 zip 会被 Rust 删除；此检查会清除 stale pending，之后正常的
    // 自动更新流程可以重新下载并从头构造 candidate，绝不复用旧 zip。
    const pending = await getPendingUpdateInfo();
    if (pending) {
      restorePendingUpdate(pending);
      if (oeaConfig.value.autoInstallUpdates) {
        void tryAutoInstall();
      } else {
        updatePopoverOpen.value = true;
      }
    }
  }

  // 扫描结束后若有待安装更新且开启自动安装，自动触发（下载完成时扫描运行中也生效）。
  await onAppStatus((status) => {
    if (!status.running) {
      void tryAutoInstall();
    }
  });

  // 检查更新（自动下载按配置触发）。
  await checkUpdate();
}

/** Rust 启动恢复的最小返回值：完成一次资源事务，或没有已完成的事务。 */
type StartupUpdateResult = 'completed' | null;

/** 查询并消费本次启动是否完成了一个更新事务。 */
async function consumeStartupUpdateResult(): Promise<StartupUpdateResult> {
  try {
    return await invoke<StartupUpdateResult>('consume_startup_update_result');
  } catch (error) {
    // 兼容尚未带启动恢复 command 的开发后端；正式后端会始终提供该只读 command。
    logWarn(`读取启动更新结果失败: ${String(error)}`);
    return null;
  }
}

/** 将 pending metadata 恢复为下载完成态。 */
function restorePendingUpdate(pending: PendingUpdateInfo): void {
  downloadState.value = {
    status: 'completed',
    update: {
      downloadedPackagePath: pending.downloadSavePath,
      versionName: pending.versionName,
      releaseNote: pending.releaseNote,
    },
  };
}

/** 满足条件时自动开始安装：下载完成 + 未在安装 + 开启自动安装 + 扫描空闲。 */
export async function tryAutoInstall(): Promise<void> {
  if (
    downloadState.value.status !== 'completed' ||
    installStatus.value !== UpdateInstallStatus.Idle ||
    !oeaConfig.value.autoInstallUpdates ||
    appStatus.value.running ||
    isInstalling
  ) {
    return;
  }
  await startInstall();
}

/** 安装启动结果：命令已接受、流程被条件阻止，或安装失败。 */
export type InstallStartResult = 'started' | 'skipped' | 'failed';

/** 开始安装（自动触发与手动「立即安装」共用；扫描任务运行中拒绝）。 */
export async function startInstall(): Promise<InstallStartResult> {
  if (isInstalling || downloadState.value.status !== 'completed') {
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

  const zipPath = downloadState.value.update.downloadedPackagePath;

  isInstalling = true;
  installStatus.value = UpdateInstallStatus.Installing;
  installError.value = null;
  installStage.value = null;
  showInstallModal.value = true;
  updatePopoverOpen.value = false;

  let unlisten: (() => void) | null = null;
  try {
    // 先订阅阶段事件，避免 Rust 在构造 candidate 的早期阶段完成得太快而丢失文案。
    unlisten = await listen<UpdateInstallStageEvent>('update-install-stage', (event) => {
      installStage.value = event.payload.stage;
    });
    // Rust 会构造 candidate、发布 transaction 并启动 helper。helper 接管后当前进程
    // 退出，所以这里没有成功后的 relaunch，也没有可供前端继续编排的细粒度 command。
    await invoke('install_update', { packagePath: zipPath });
    return 'started';
  } catch (error) {
    // Rust 在安装准备或 helper 启动失败时删除 zip；保留错误提示，但把下载态清空，
    // 使下一次重试从下载阶段开始，不会误用已经消费过的 zip。
    clearDownloadedUpdateState();
    handleInstallFailure(error);
    return 'failed';
  } finally {
    unlisten?.();
    isInstalling = false;
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
  clearDownloadedUpdateState();

  if (updateCheckState.value.status === 'available') {
    await startDownload();
  } else {
    // 没有可直接重用的检查结果时让正常检查流程重新取得下载信息。
    await checkUpdate();
  }
}

/** 关闭安装弹窗（仅失败 / 完成（重启失败）可关闭；安装中不可关闭由弹窗控制）。 */
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

/** 安装失败统一处理：置状态 + 日志。 */
function handleInstallFailure(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  installStatus.value = UpdateInstallStatus.Failed;
  installError.value = message;
  logError(`更新安装失败: ${message}`);
}

/** 清除已消费的 zip 及其前端状态，让下次安装从下载阶段开始。 */
function clearDownloadedUpdateState(): void {
  clearPendingUpdateInfo();
  downloadState.value = { status: 'idle' };
}

/** 下载失败统一处理：置状态 + 日志 + toast。 */
function handleDownloadFailure(error: unknown, fallbackTitle: string): void {
  const message = error instanceof Error ? error.message : String(error);
  downloadState.value = { status: 'failed' };
  logError(`${fallbackTitle}: ${message}`);
  useToast().add({
    title: fallbackTitle,
    description: message,
    icon: 'i-lucide-triangle-alert',
    color: 'error',
  });
}
