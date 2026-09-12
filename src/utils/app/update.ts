import { oeaVersion } from '@/main';
import {
  MirrorchyanResourcesLatestResponse,
  MirrorchyanResourcesLatestResponseData,
} from '@/types/mirrorchyan';
import { OemStableManifest } from '@/types/oem';
import { UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import {
  GitHubRelease,
  PendingUpdateInfo,
  PreparedUpdate,
  UpdateCheckResult,
  UpdateCheckStatus,
  UpdateCompleteInfo,
  UpdateDownloadProgress,
  UpdateDownloadStatus,
  UpdateInstallStageEvent,
  UpdateInstallStage,
  UpdateInstallStatus,
  UpdatePackageType,
} from '@/types/update';
import { appStatus } from '@/utils/app/appStatus';
import { mirrorchyanCdk, oeaConfig } from '@/utils/app/config';
import { logError, logInfo, logWarn, onAppStatus } from '@/utils/tauri';
import { updatePopoverOpen } from '@/utils/uiState';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { join } from '@tauri-apps/api/path';
import { ClientOptions, fetch } from '@tauri-apps/plugin-http';
import { arch, platform, version } from '@tauri-apps/plugin-os';
import { gt } from 'semver';
import { ref } from 'vue';

/** MirrorChyan 资源 ID（与发布侧 rid 一致，大写）。 */
const RESOURCE_ID = 'OEA';
/** MirrorChyan 检查更新 API 主备双站。 */
const CHECK_URL_BASES = [
  `https://mirrorchyan.com/api/resources/${RESOURCE_ID}/latest`,
  `https://mirrorchyan.net/api/resources/${RESOURCE_ID}/latest`,
];
/** GitHub 仓库（Logical-Byte/open-endfield-assistant）。 */
const GITHUB_OWNER = 'Logical-Byte';
const GITHUB_REPO = 'open-endfield-assistant';
const GITHUB_RELEASES_URL = `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases`;
/** OEM 稳定版元数据接口。 */
const OEM_STABLE_MANIFEST_URL = 'https://package.oem.re/channels/oea/stable.json';

/**
 * 根据实际系统信息生成更新请求 UA（`OEA/<版本> (Windows NT <major.minor>; Win64; x64)`）。
 *
 * - 版本取系统真实版本的前两段（Win10/Win11 的 NT 版本均为 `10.0`，与浏览器 UA 一致）；
 * - 架构用插件返回的进程架构（x64 构建在 ARM64 机器上仍为 `x86_64`，与浏览器 UA 约定一致）。
 */
function buildUpdateUserAgent(): string {
  const nt = version().split('.').slice(0, 2).join('.');
  const osArch = arch();
  const archToken = osArch === 'x86_64' ? 'Win64; x64' : osArch === 'aarch64' ? 'ARM64' : osArch;
  return `OEA/${oeaVersion} (Windows NT ${nt}; ${archToken})`;
}

/** 构造检查更新请求 URL（主备站共用同一套查询参数；query 必须用 URLSearchParams，禁止字符串拼接）。 */
function buildCheckUpdateUrl(base: string): URL {
  const url = new URL(base);
  url.searchParams.set('current_version', `v${oeaVersion}`);
  url.searchParams.set('user_agent', 'oea_client');
  url.searchParams.set('channel', 'stable');
  url.searchParams.set('os', 'windows');
  url.searchParams.set('arch', 'amd64');
  const cdk = mirrorchyanCdk.value.trim();
  if (oeaConfig.value.updateSource === UpdateSource.Mirrorchyan && cdk) {
    url.searchParams.set('cdk', cdk);
  }
  return url;
}

/** 检查更新结果（弹 Popover 的依据）。 */
export const updateCheckResult = ref<UpdateCheckResult>({ status: UpdateCheckStatus.Idle });
/** 下载阶段状态。 */
export const downloadStatus = ref<UpdateDownloadStatus>(UpdateDownloadStatus.Idle);
/** 下载进度（驱动 Popover 进度条）。 */
export const downloadProgress = ref<UpdateDownloadProgress>({
  downloadedSize: 0,
  totalSize: 0,
  speed: 0,
  progress: 0,
});
/** 下载包实际保存路径（下载成功后的唯一事实来源，安装调用依赖它）。 */
export const downloadSavePath = ref<string | null>(null);
/** 当前已就绪的下载信息（URL/sha256 等）。 */
export const preparedUpdate = ref<PreparedUpdate | null>(null);
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

/** 当前下载会话编号（过滤旧任务的迟到进度事件）。 */
let currentSessionId: number | null = null;
/** 下载互斥：同一时间只允许一个下载任务。 */
let isDownloading = false;
/** 是否为用户主动取消（取消不视为错误）。 */
let downloadCancelled = false;
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

/** Rust `download-progress` 事件 payload（camelCase）。 */
interface DownloadProgressEventPayload extends UpdateDownloadProgress {
  sessionId: number;
}

/** Rust `download_file` 命令返回值。 */
interface DownloadResultPayload {
  sessionId: number;
  actualSavePath: string;
  detectedFilename: string | null;
}

/**
 * 执行一次检查更新（启动自动检查与设置页手动检查共用）。
 *
 * 检查固定请求 MirrorChyan（主备双站），无论「更新源」设置为何：更新源只决定
 * 后续下载从 Mirror酱、OEM 还是 GitHub 拉取，不影响「是否可更新」的判定。
 */
export async function checkUpdate(): Promise<void> {
  // 若当前正在检查更新，则忽略本次请求（避免重复请求）。
  if (updateCheckResult.value.status === UpdateCheckStatus.Checking) {
    return;
  }

  updateCheckResult.value = { status: UpdateCheckStatus.Checking };
  let maybePayload: MirrorchyanResourcesLatestResponse | undefined = undefined;
  try {
    const headers: Record<string, string> = {
      'User-Agent': buildUpdateUserAgent(),
      Accept: 'application/json',
    };

    // 解析检查请求使用的代理（System 模式从注册表读取，Custom 用配置 URL）。
    const proxyInit = await buildProxyClientOptions();

    // 依次尝试主站与备站。
    let lastError: Error | undefined = undefined;
    for (const base of CHECK_URL_BASES) {
      try {
        const response = await fetch(buildCheckUpdateUrl(base), {
          method: 'GET',
          headers,
          ...proxyInit,
        });
        const parsed: MirrorchyanResourcesLatestResponse = await response.json();
        maybePayload = parsed;
        if (parsed.code === 0) {
          break;
        }
        lastError = new Error(`Mirror 酱服务返回错误: code=${parsed.code}, msg=${parsed.msg}`);
        logWarn(`${base} 返回错误 code=${parsed.code}，尝试备用站`);
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        logWarn(`${base} 请求失败: ${lastError.message}`);
      }
    }

    if (!maybePayload) {
      throw new Error(
        `检查更新请求失败，请检查网络连接或代理设置，或稍后重试。\n${lastError?.message}`,
      );
    }

    // 检查业务错误码，若非 0 则视为失败。
    if (maybePayload.code !== 0) {
      throw buildMirrorchyanError(maybePayload.code, maybePayload.msg);
    }
    if (!maybePayload.data) {
      throw new Error('检查更新服务响应异常，请稍后重试');
    }

    const data = maybePayload.data;
    const latestVersion = data.version_name;
    const hasUpdate = isNewer(latestVersion, oeaVersion);

    if (hasUpdate) {
      logWarn(`检查更新：有新版本可用，当前 v${oeaVersion}，最新 ${latestVersion}`);
      updateCheckResult.value = { status: UpdateCheckStatus.HasUpdate, result: maybePayload };
      updatePopoverOpen.value = true;
      // 「自动下载更新」开启时直接开始下载（无需用户点击）。
      // 安装进行中不触发自动下载，避免与续装流程互相干扰。
      if (oeaConfig.value.autoDownloadUpdates && installStatus.value === UpdateInstallStatus.Idle) {
        void startDownload(maybePayload.data);
      }
    } else {
      logInfo(`检查更新：已是最新版本 v${oeaVersion}`);
      updateCheckResult.value = { status: UpdateCheckStatus.NoUpdate, result: maybePayload };
    }
  } catch (error) {
    const errorInstance = error instanceof Error ? error : new Error(String(error));
    updateCheckResult.value = {
      status: UpdateCheckStatus.Error,
      error: errorInstance,
      result: maybePayload,
    };
    // 若检查更新失败，弹出 Popover 提示用户手动检查（避免用户错过更新）。
    updatePopoverOpen.value = true;
    logError(`检查更新失败: ${errorInstance.message}`);
  }
}

/**
 * 开始下载更新（自动下载与手动「立即更新」共用）。
 *
 * 流程：准备下载信息（决定源与校验信息）→ 监听 `download-progress` 进度事件（按 session 过滤）→
 * 调 Rust `download_file` 流式下载 → 成功后保存实际路径。
 *
 * 注意：本函数会一直等到下载结束（成功 / 失败 / 取消）才返回，不会在开始下载后立即返回。
 * Rust 端 `download_file` 会流式读完整响应体（含磁盘写入与 sha256 校验）后才 resolve，
 * 下载期间的状态由独立的 `download-progress` 事件上报。
 */
export async function startDownload(
  checkUpdateData: MirrorchyanResourcesLatestResponseData,
): Promise<void> {
  if (isDownloading) {
    return;
  }
  // 互斥锁必须在准备阶段之前占用：`prepareDownload` 含 GitHub API 请求（较慢），
  // 若等准备完成后再上锁，「自动下载」与手快点击的「立即更新」会双发，
  // 后一个下载会让前一个因 session 失效而报"下载已取消"。
  isDownloading = true;
  downloadCancelled = false;
  currentSessionId = null;
  downloadStatus.value = UpdateDownloadStatus.Downloading;
  updatePopoverOpen.value = true;

  let unlisten: (() => void) | null = null;
  try {
    // 准备下载信息。
    const prepared = await prepareDownload(checkUpdateData);
    if (!prepared) {
      handleDownloadFailure(new Error('未获取到可用的下载链接'), '准备下载失败');
      return;
    }
    // 准备阶段内被取消（防御：取消按钮只在下载中显示，正常不会触发）。
    if (downloadCancelled) {
      downloadStatus.value = UpdateDownloadStatus.Idle;
      return;
    }
    preparedUpdate.value = prepared;
    downloadProgress.value = {
      downloadedSize: 0,
      totalSize: prepared.fileSize ?? 0,
      speed: 0,
      progress: 0,
    };

    const saveDir = await invoke<string>('get_download_dir');
    const defaultName = `OEA-windows-x86_64-${prepared.versionName}.zip`;
    const savePath = await join(saveDir, prepared.filename ?? defaultName);

    unlisten = await listen<DownloadProgressEventPayload>('download-progress', (event) => {
      // 只处理当前 session 的进度事件，忽略旧任务的迟到事件。
      if (currentSessionId !== null && event.payload.sessionId !== currentSessionId) {
        return;
      }
      if (currentSessionId === null) {
        currentSessionId = event.payload.sessionId;
      }
      if (downloadCancelled) {
        return;
      }
      downloadProgress.value = event.payload;
    });

    const { updateProxyMode, updateProxyUrl } = oeaConfig.value;
    const proxyMode =
      prepared.source === UpdateSource.Mirrorchyan ? UpdateProxyMode.None : updateProxyMode;
    // 阻塞直到下载结束：Rust `download_file` 流式读完整响应体、写完盘并校验 sha256 后才返回，
    // 不会在开始下载后立即返回；期间进度由上面的 `download-progress` 事件上报。
    const result = await invoke<DownloadResultPayload>('download_file', {
      request: {
        url: prepared.url,
        savePath,
        totalSize: prepared.fileSize ?? null,
        expectedSha256: prepared.sha256 ?? null,
        proxyMode,
        proxyUrl: proxyMode === UpdateProxyMode.Custom ? updateProxyUrl : null,
        accept: prepared.source === UpdateSource.Github ? 'application/octet-stream' : null,
        userAgent: buildUpdateUserAgent(),
      },
    });

    // 取消可能在 Rust 收尾阶段才到达（下载实际已完成）：尊重用户意图，不进入已完成态。
    if (downloadCancelled) {
      logInfo('下载已被用户取消（下载已基本完成）');
      downloadStatus.value = UpdateDownloadStatus.Idle;
      return;
    }

    currentSessionId = result.sessionId;
    downloadSavePath.value = result.actualSavePath;
    downloadStatus.value = UpdateDownloadStatus.Completed;
    logInfo(`更新下载完成: ${result.actualSavePath}`);
    // 保存待安装信息，供正常 helper/v2 启动流程展示完成提示；再按「自动安装更新」触发安装。
    savePendingUpdateInfo({
      previousVersion: oeaVersion,
      versionName: prepared.versionName,
      releaseNote: prepared.releaseNote,
      downloadSavePath: result.actualSavePath,
      fileSize: prepared.fileSize,
      updateType: prepared.updateType,
      downloadSource: prepared.source,
      timestamp: Date.now(),
    });
    void tryAutoInstall();
  } catch (error) {
    if (downloadCancelled) {
      logInfo('下载已被用户取消');
      downloadStatus.value = UpdateDownloadStatus.Idle;
    } else {
      handleDownloadFailure(error, '下载失败');
    }
  } finally {
    unlisten?.();
    isDownloading = false;
  }
}

/** 取消当前下载（Rust 置取消标志，临时文件由守卫清理）。 */
export async function cancelDownload(): Promise<void> {
  if (!isDownloading || downloadStatus.value === UpdateDownloadStatus.Cancelling) {
    return;
  }
  downloadCancelled = true;
  // 置「正在取消」状态：Rust 收尾期间「立即更新」与下载设置按钮会被隐藏，
  // 待 `startDownload` 的 finally 复位 `isDownloading` 后回到 Idle。
  downloadStatus.value = UpdateDownloadStatus.Cancelling;
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
  preparedUpdate.value = {
    url: '',
    source: pending.downloadSource ?? UpdateSource.Mirrorchyan,
    updateType: pending.updateType,
    versionName: pending.versionName,
    releaseNote: pending.releaseNote,
    fileSize: pending.fileSize,
  };
  downloadSavePath.value = pending.downloadSavePath;
  downloadStatus.value = UpdateDownloadStatus.Completed;
}

/** 满足条件时自动开始安装：下载完成 + 未在安装 + 开启自动安装 + 扫描空闲。 */
export async function tryAutoInstall(): Promise<void> {
  if (
    downloadStatus.value !== UpdateDownloadStatus.Completed ||
    installStatus.value !== UpdateInstallStatus.Idle ||
    !oeaConfig.value.autoInstallUpdates ||
    appStatus.value.running ||
    isInstalling
  ) {
    return;
  }
  await startInstall();
}

/** 开始安装（自动触发与手动「立即安装」共用；扫描任务运行中拒绝）。 */
export async function startInstall(): Promise<void> {
  if (isInstalling) {
    return;
  }
  if (appStatus.value.running) {
    useToast().add({
      title: '扫描任务运行中',
      description: '扫描结束后将自动安装更新',
      icon: 'i-lucide-info',
      color: 'info',
    });
    return;
  }

  const zipPath = downloadSavePath.value;
  const prepared = preparedUpdate.value;
  if (!zipPath || !prepared) {
    clearDownloadedUpdateState();
    handleInstallFailure(new Error('缺少下载包信息，请重新下载'));
    return;
  }

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
  } catch (error) {
    // Rust 在安装准备或 helper 启动失败时删除 zip；保留错误提示，但把下载态清空，
    // 使下一次重试从下载阶段开始，不会误用已经消费过的 zip。
    clearDownloadedUpdateState();
    handleInstallFailure(error);
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

  const result = updateCheckResult.value;
  if (result.status === UpdateCheckStatus.HasUpdate && result.result.data) {
    await startDownload(result.result.data);
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
  downloadSavePath.value = null;
  preparedUpdate.value = null;
  downloadStatus.value = UpdateDownloadStatus.Idle;
  downloadProgress.value = {
    downloadedSize: 0,
    totalSize: 0,
    speed: 0,
    progress: 0,
  };
}

/**
 * 下载源决策：
 * 1. 明确选择 GitHub → 从 GitHub 匹配 tag 与资产（含 digest）；
 * 2. 更新源为 MirrorChyan 且 CDK 已填写、MirrorChyan 给了 url → 用 MirrorChyan；
 * 3. 其他情况（选择 OEM、MirrorChyan 未填写 CDK 或未给 url）→ 获取 OEM 稳定元数据并使用 OEM 下载。
 * OEM 元数据请求失败或版本不一致时直接抛错，不回退到其他下载源。
 */
async function prepareDownload(
  data: MirrorchyanResourcesLatestResponseData,
): Promise<PreparedUpdate | null> {
  const cdk = mirrorchyanCdk.value.trim();
  const versionName = data.version_name;

  if (oeaConfig.value.updateSource === UpdateSource.Mirrorchyan && cdk && data.url) {
    return {
      url: data.url,
      sha256: data.sha256,
      fileSize: data.filesize,
      source: UpdateSource.Mirrorchyan,
      updateType:
        data.update_type === 'incremental' ? UpdatePackageType.Incremental : UpdatePackageType.Full,
      versionName,
      releaseNote: data.release_note,
    };
  }

  if (oeaConfig.value.updateSource !== UpdateSource.Github) {
    const manifest = await resolveOemDownload(versionName);
    return {
      url: manifest.url,
      sha256: manifest.sha256,
      fileSize: manifest.size,
      filename: manifest.filename,
      source: UpdateSource.Oem,
      updateType: UpdatePackageType.Full,
      versionName: manifest.tag,
      releaseNote: data.release_note,
    };
  }

  const github = await resolveGithubDownload(versionName);
  if (!github) {
    return null;
  }
  return {
    url: github.url,
    sha256: github.sha256,
    fileSize: github.fileSize,
    filename: github.filename,
    source: UpdateSource.Github,
    updateType: UpdatePackageType.Full,
    versionName,
    releaseNote: data.release_note,
  };
}

/**
 * 获取 OEM 稳定版元数据。
 *
 * OEM 版本必须与 Mirror酱本次检查返回的版本一致，否则中断下载。
 */
async function resolveOemDownload(versionName: string): Promise<OemStableManifest> {
  const init: RequestInit & ClientOptions = {
    method: 'GET',
    headers: {
      'User-Agent': buildUpdateUserAgent(),
      Accept: 'application/json',
    },
    ...(await buildProxyClientOptions()),
  };

  let response: Response;
  try {
    response = await fetch(OEM_STABLE_MANIFEST_URL, init);
  } catch (error) {
    throw new Error(
      `OEM 更新元数据请求失败: ${error instanceof Error ? error.message : String(error)}`,
    );
  }

  if (!response.ok) {
    throw new Error(`OEM 更新元数据请求失败（HTTP ${response.status}），已中断下载`);
  }

  let manifest: OemStableManifest;
  try {
    manifest = await response.json();
  } catch (error) {
    throw new Error(`OEM 更新元数据不是有效 JSON，已中断下载: ${String(error)}`);
  }

  const mirrorchyanVersion = versionName.trim().replace(/^v/i, '');
  if (manifest.version !== mirrorchyanVersion) {
    throw new Error(
      `OEM 与 Mirror酱版本不一致（OEM: ${manifest.tag}，Mirror酱: ${versionName}），已中断下载`,
    );
  }

  logInfo(`OEM 与 Mirror酱版本一致: ${manifest.tag}，下载地址: ${manifest.url}`);
  return manifest;
}

/**
 * 从 GitHub Releases 获取目标版本的下载资产。
 *
 * - 按已知 tag（`v<新版本>`）直接请求「按 tag 获取 release」端点，
 *   避免列表接口只返回前 100 条导致的分页遗漏；
 * - 匹配资产：优先精确匹配 `OEA-<平台>-<架构>-v<版本>.zip`，无精确匹配时取体积最大的 zip 资产；
 * - 校验信息：取 asset `digest`（须为 `sha256:<hex>` 格式，否则视为无校验信息）；
 * - 下载地址：使用 asset 的 API `url`（`/releases/assets/{id}`），
 *   Rust 下载端会带 `Accept: application/octet-stream` 请求并跟随 302 重定向。
 */
async function resolveGithubDownload(
  versionName: string,
): Promise<{ url: string; sha256?: string; fileSize: number; filename: string } | null> {
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': buildUpdateUserAgent(),
  };

  const init: RequestInit & ClientOptions = {
    method: 'GET',
    headers,
    ...(await buildProxyClientOptions()),
  };

  let response: Response;
  try {
    response = await fetch(`${GITHUB_RELEASES_URL}/tags/${encodeURIComponent(versionName)}`, init);
  } catch (error) {
    throw new Error(
      `GitHub API 请求失败: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (response.status === 404) {
    throw new Error(`GitHub 上未找到版本 ${versionName} 的 Release`);
  }
  if (!response.ok) {
    throw new Error(`GitHub API 错误（HTTP ${response.status}），GitHub 下载暂不可用`);
  }

  const release: GitHubRelease = await response.json();

  // 匹配资产：优先精确匹配 `OEA-<平台>-<架构>-<版本>.zip`，无精确匹配时取体积最大的 zip 资产。
  const exactName = `OEA-${platform()}-${arch()}-${versionName}.zip`;
  const candidates = release.assets.filter((asset) => asset.name.toLowerCase().endsWith('.zip'));
  let asset = candidates.find((item) => item.name === exactName) ?? null;
  if (!asset && candidates.length > 0) {
    asset = candidates.reduce((largest, item) => (item.size > largest.size ? item : largest));
  }
  if (!asset) {
    throw new Error('GitHub Release 中未找到 OEA-windows-x86_64 的 zip 资产');
  }

  // GitHub 资产 digest 期望为 `sha256:<sha256>` 格式，格式不符时视为没有校验信息。
  const digestMatch = asset.digest?.match(/^sha256:([0-9a-f]{64})$/i);
  const sha256 = digestMatch?.[1]?.toLowerCase();
  if (!sha256) {
    logWarn(`GitHub 资产缺少合法的 sha256 digest，跳过 sha256 校验: ${asset.name}`);
  }
  return {
    url: asset.url,
    sha256,
    fileSize: asset.size,
    filename: asset.name,
  };
}

/** 构建检查/API 请求的代理配置（System 模式从注册表解析一次）。 */
async function buildProxyClientOptions(): Promise<ClientOptions> {
  const { updateProxyMode, updateProxyUrl } = oeaConfig.value;
  if (updateProxyMode === UpdateProxyMode.Custom && updateProxyUrl) {
    return { proxy: { all: updateProxyUrl } };
  }
  if (updateProxyMode === UpdateProxyMode.System) {
    const systemProxy = await invoke<string | null>('resolve_system_proxy');
    if (systemProxy) {
      return { proxy: { all: systemProxy } };
    }
  }
  return {};
}

/** 下载失败统一处理：置状态 + 日志 + toast。 */
function handleDownloadFailure(error: unknown, fallbackTitle: string): void {
  const message = error instanceof Error ? error.message : String(error);
  downloadStatus.value = UpdateDownloadStatus.Failed;
  logError(`${fallbackTitle}: ${message}`);
  useToast().add({
    title: fallbackTitle,
    description: message,
    icon: 'i-lucide-triangle-alert',
    color: 'error',
  });
}

/** MirrorChyan 业务错误码对应的用户可读描述（见 `temp/mirrorchyan-error-code.md`）。 */
const BUSINESS_ERROR_MESSAGES: Record<number, string> = {
  1001: 'Mirror酱：请求参数不正确，请联系作者',
  7001: '您的 Mirror酱 CDK 已过期',
  7002: '您的 Mirror酱 CDK 错误，请检查输入是否正确',
  7003: '您的 Mirror酱 CDK 今日下载次数已达上限',
  7004: '您的 Mirror酱 CDK 类型与待下载资源不匹配',
  7005: '您的 Mirror酱 CDK 已被封禁',
  8001: 'Mirror酱：对应架构和系统下的资源不存在，请联系作者',
  8002: 'Mirror酱：错误的系统参数，请联系作者',
  8003: 'Mirror酱：错误的架构参数，请联系作者',
  8004: 'Mirror酱：错误的更新通道参数，请联系作者',
};

/** 将 MirrorChyan 返回的错误码转换为用户可读的 `Error`。 */
function buildMirrorchyanError(code: number, msg: string): Error {
  if (code < 0) {
    return new Error(`Mirror 酱服务出现异常，请稍后重试或联系技术支持: ${msg}`);
  }
  const friendly = BUSINESS_ERROR_MESSAGES[code];
  if (friendly) {
    return new Error(friendly);
  }
  // `code === 1`（UNDIVIDED）等未区分的业务错误，以响应体 `msg` 为准。
  return new Error(msg || `未知错误（${code}）`);
}

/** 判断 latest 是否比 current 更新；任一侧无法解析时保守视为无更新。 */
function isNewer(latest: string, current: string): boolean {
  try {
    return gt(latest, current);
  } catch (error) {
    logError(`检查更新：版本号比较失败，视为无更新: ${JSON.stringify({ latest, current, error })}`);
    return false;
  }
}
