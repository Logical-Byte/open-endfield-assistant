import { oeaVersion } from '@/main';
import { MirrorchyanResourcesLatestResponse } from '@/types/mirrorchyan';
import { UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import {
  ChangesJson,
  GitHubRelease,
  PendingUpdateInfo,
  PreparedUpdate,
  UpdateCheckResult,
  UpdateCheckStatus,
  UpdateCompleteInfo,
  UpdateDownloadProgress,
  UpdateDownloadStatus,
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
/** 下载包实际保存路径（下载成功后的唯一事实来源，安装/续装都依赖它）。 */
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
const UPDATE_COMPLETE_KEY = 'oea-update-complete';

/** 安装阶段 → 用户可读文案。 */
const INSTALL_STAGE_LABELS: Record<UpdateInstallStage, string> = {
  [UpdateInstallStage.BackingUp]: '备份配置',
  [UpdateInstallStage.Extracting]: '解压更新包',
  [UpdateInstallStage.Checking]: '检查更新包类型',
  [UpdateInstallStage.ApplyingIncremental]: '应用增量更新',
  [UpdateInstallStage.ApplyingFull]: '应用全量更新',
  [UpdateInstallStage.CleaningUp]: '清理临时文件',
  [UpdateInstallStage.Done]: '安装完成',
};

/** 安装阶段文案（供弹窗展示）。 */
export function installStageLabel(stage: UpdateInstallStage): string {
  return INSTALL_STAGE_LABELS[stage];
}

/** 保存待安装更新信息（下载成功后调用，崩溃/重启后可续装）。 */
function savePendingUpdateInfo(info: PendingUpdateInfo): void {
  try {
    localStorage.setItem(PENDING_UPDATE_KEY, JSON.stringify(info));
  } catch (error) {
    logWarn(`保存待安装更新信息失败: ${String(error)}`);
  }
}

/** 读取待安装更新信息；zip 已被删除时自动清除并返回 `null`。 */
async function getPendingUpdateInfo(): Promise<PendingUpdateInfo | null> {
  try {
    const raw = localStorage.getItem(PENDING_UPDATE_KEY);
    if (!raw) {
      return null;
    }
    const info = JSON.parse(raw) as PendingUpdateInfo;
    if (!info.downloadSavePath) {
      localStorage.removeItem(PENDING_UPDATE_KEY);
      return null;
    }
    const exists = await invoke<boolean>('pending_package_exists', {
      savePath: info.downloadSavePath,
    });
    if (!exists) {
      logWarn('待安装的更新包已被删除，清除待安装信息');
      localStorage.removeItem(PENDING_UPDATE_KEY);
      return null;
    }
    return info;
  } catch (error) {
    logWarn(`读取待安装更新信息失败: ${String(error)}`);
    localStorage.removeItem(PENDING_UPDATE_KEY);
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

/** 保存更新完成信息（重启前写入，新进程启动时展示）。 */
function saveUpdateCompleteInfo(info: UpdateCompleteInfo): void {
  try {
    localStorage.setItem(UPDATE_COMPLETE_KEY, JSON.stringify(info));
  } catch (error) {
    logWarn(`保存更新完成信息失败: ${String(error)}`);
  }
}

/** 读取并清除更新完成信息（重启后展示用）。 */
function consumeUpdateCompleteInfo(): UpdateCompleteInfo | null {
  try {
    const raw = localStorage.getItem(UPDATE_COMPLETE_KEY);
    if (!raw) {
      return null;
    }
    localStorage.removeItem(UPDATE_COMPLETE_KEY);
    return JSON.parse(raw) as UpdateCompleteInfo;
  } catch (error) {
    logWarn(`读取更新完成信息失败: ${String(error)}`);
    localStorage.removeItem(UPDATE_COMPLETE_KEY);
    return null;
  }
}

/** Rust `download-progress` 事件 payload（camelCase）。 */
interface DownloadProgressEventPayload extends UpdateDownloadProgress {
  sessionId: number;
}

/** Rust `download_update` 命令返回值。 */
interface DownloadResultPayload {
  sessionId: number;
  actualSavePath: string;
  detectedFilename: string | null;
}

/**
 * 执行一次检查更新（启动自动检查与设置页手动检查共用）。
 *
 * 检查固定请求 MirrorChyan（主备双站），无论「更新源」设置为何：更新源只决定
 * 后续下载从镜像还是 GitHub 拉取，不影响「是否可更新」的判定（见设计文档 v4）。
 */
export async function checkUpdate(): Promise<void> {
  // 若当前正在检查更新，则忽略本次请求（避免重复请求）。
  if (updateCheckResult.value.status === UpdateCheckStatus.Checking) {
    return;
  }

  updateCheckResult.value = { status: UpdateCheckStatus.Checking };
  let maybePayload: MirrorchyanResourcesLatestResponse | undefined = undefined;
  try {
    // 构造请求参数（主备站共用同一套参数）。
    const url = new URL(CHECK_URL_BASES[0]);
    url.searchParams.set('current_version', `v${oeaVersion}`);
    url.searchParams.set('user_agent', 'oea_client');
    url.searchParams.set('channel', 'stable');
    url.searchParams.set('os', 'windows');
    url.searchParams.set('arch', 'amd64');
    const cdk = mirrorchyanCdk.value.trim();
    if (cdk) {
      url.searchParams.set('cdk', cdk);
    }
    const params = url.searchParams.toString();

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
        const response = await fetch(`${base}?${params}`, {
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
        void startDownload();
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
 * 调 Rust `download_update` 流式下载 → 成功后保存实际路径。
 *
 * 注意：本函数会一直等到下载结束（成功 / 失败 / 取消）才返回，不会在开始下载后立即返回。
 * Rust 端 `download_update` 会流式读完整响应体（含磁盘写入与 sha256 校验）后才 resolve，
 * 下载期间的状态由独立的 `download-progress` 事件上报。
 */
export async function startDownload(): Promise<void> {
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

  let unlisten: (() => void) | null = null;
  try {
    // 准备下载信息：MirrorChyan 直连 / GitHub 匹配资产（含 digest）。
    const prepared = await prepareDownload();
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

    const saveDir = await invoke<string>('get_update_download_dir');
    const defaultName = `OEA-windows-x86_64-v${prepared.versionName.replace(/^v/i, '')}.zip`;
    const savePath = `${saveDir}/${prepared.filename ?? defaultName}`;

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
      downloadProgress.value = {
        downloadedSize: event.payload.downloadedSize,
        totalSize: event.payload.totalSize,
        speed: event.payload.speed,
        progress: event.payload.progress,
      };
    });

    const { updateProxyMode, updateProxyUrl } = oeaConfig.value;
    const proxyMode =
      prepared.source === UpdateSource.Github ? updateProxyMode : UpdateProxyMode.None;
    // 阻塞直到下载结束：Rust `download_update` 流式读完整响应体、写完盘并校验 sha256 后才返回，
    // 不会在开始下载后立即返回；期间进度由上面的 `download-progress` 事件上报。
    const result = await invoke<DownloadResultPayload>('download_update', {
      url: prepared.url,
      savePath,
      totalSize: prepared.fileSize ?? null,
      expectedSha256: prepared.sha256 ?? null,
      proxyMode,
      proxyUrl: proxyMode === UpdateProxyMode.Custom ? updateProxyUrl : null,
      authToken: prepared.source === UpdateSource.Github ? __OEA_GITHUB_TOKEN__ : null,
      accept: prepared.source === UpdateSource.Github ? 'application/octet-stream' : null,
      userAgent: buildUpdateUserAgent(),
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
    // 保存待安装信息，崩溃/重启后可续装；再按「自动安装更新」触发安装。
    savePendingUpdateInfo({
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
 * 启动时序（v4 §2）：更新完成弹窗 → 待安装续装 → 启动清扫 → 检查更新。
 * 由 `App.vue` 在配置加载完成后调用。
 */
export async function initUpdateState(): Promise<void> {
  // 1. 重启后展示「更新完成」（绿色便携 zip 无 requireVersionCheck 场景）。
  const complete = consumeUpdateCompleteInfo();
  if (complete) {
    justUpdatedInfo.value = complete;
    showInstallModal.value = true;
    clearPendingUpdateInfo();
  }

  // 2. 启动清扫（崩溃残留 / 上一轮 old / 半成品下载）。
  //    必须在待安装续装之前执行，避免与安装流程并发操作 `cache/old`。
  try {
    await invoke('cleanup_stale_update_files');
  } catch (error) {
    logWarn(`启动清扫更新残留失败: ${String(error)}`);
  }

  // 3. 恢复上次下载完成但未安装的更新。
  const pending = await getPendingUpdateInfo();
  if (pending) {
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
    if (oeaConfig.value.autoInstallUpdates) {
      void tryAutoInstall();
    } else {
      updatePopoverOpen.value = true;
    }
  }

  // 4. 扫描结束后若有待安装更新且开启自动安装，自动触发（下载完成时扫描运行中也生效）。
  await onAppStatus((status) => {
    if (!status.running) {
      void tryAutoInstall();
    }
  });

  // 5. 检查更新（自动下载按配置触发）。
  await checkUpdate();
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
    handleInstallFailure(new Error('缺少下载包信息，请重新下载'));
    return;
  }

  isInstalling = true;
  installStatus.value = UpdateInstallStatus.Installing;
  installError.value = null;
  installStage.value = null;
  showInstallModal.value = true;
  updatePopoverOpen.value = false;

  try {
    await invoke('set_update_installing', { installing: true });
    try {
      await runInstallSteps(zipPath, prepared);
    } finally {
      // 安装成功会 relaunch，此调用可能来不及返回；失败时确保标志复位
      await invoke('set_update_installing', { installing: false }).catch(() => {});
    }

    // 应用已成功：重启失败不回滚，仅提示手动重启。
    installStage.value = UpdateInstallStage.Done;
    try {
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
    } catch (error) {
      logError(`自动重启失败，请手动重启应用: ${String(error)}`);
      installError.value = '安装已完成，但自动重启失败，请手动重启应用';
    }
  } catch (error) {
    // 安装失败：尽力回滚（old 中保留本次移走的全部旧文件，整体搬回）。
    try {
      await invoke('restore_from_old');
      logWarn('安装失败，已尽力回滚旧文件');
    } catch (rollbackError) {
      logWarn(`回滚失败（old 目录已保留旧文件）: ${String(rollbackError)}`);
    }
    handleInstallFailure(error);
  } finally {
    isInstalling = false;
  }
}

/** 执行安装步骤（备份 → 解压 → 判定 → 应用 → 清理 → 写完成状态）。 */
async function runInstallSteps(zipPath: string, prepared: PreparedUpdate): Promise<void> {
  // 1. 清空 old（保证回滚基线干净），再备份配置（失败仅 warn，Rust 已保证不报错）。
  await invoke('cleanup_old_dir');
  installStage.value = UpdateInstallStage.BackingUp;
  await invoke('backup_config');

  // 2. 解压（内部会先清理残留解压目录）。
  installStage.value = UpdateInstallStage.Extracting;
  await invoke('extract_zip', { zipPath });

  // 3. 判定增量 / 全量（以 changes.json 是否存在于解压目录为准）。
  installStage.value = UpdateInstallStage.Checking;
  const changes = await invoke<ChangesJson | null>('check_changes_json');
  if (changes) {
    installStage.value = UpdateInstallStage.ApplyingIncremental;
    await invoke('apply_incremental_update', { deleted: changes.deleted });
  } else {
    installStage.value = UpdateInstallStage.ApplyingFull;
    await invoke('apply_full_update');
  }

  // 4. 应用成功：清理（失败仅 warn，不再回滚）。
  installStage.value = UpdateInstallStage.CleaningUp;
  await invoke('cleanup_extract_dir').catch((error) => {
    logWarn(`清理解压目录失败: ${String(error)}`);
  });
  await invoke('remove_downloaded_package', { savePath: zipPath }).catch((error) => {
    logWarn(`删除更新包失败: ${String(error)}`);
  });

  // 5. 保存「更新完成」信息并清除 pending，随后由 startInstall 触发重启。
  clearPendingUpdateInfo();
  saveUpdateCompleteInfo({
    previousVersion: oeaVersion,
    newVersion: prepared.versionName,
    releaseNote: prepared.releaseNote,
    timestamp: Date.now(),
  });
  installStage.value = UpdateInstallStage.Done;
  installStatus.value = UpdateInstallStatus.Completed;
}

/** 安装失败后重试（重新走完整安装流程，幂等）。 */
export async function retryInstall(): Promise<void> {
  if (installStatus.value === UpdateInstallStatus.Failed) {
    installStatus.value = UpdateInstallStatus.Idle;
    installError.value = null;
  }
  await startInstall();
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
  }
}

/** 安装失败统一处理：置状态 + 日志。 */
function handleInstallFailure(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  installStatus.value = UpdateInstallStatus.Failed;
  installError.value = message;
  logError(`更新安装失败: ${message}`);
}

/**
 * 下载源决策（设计文档 v4 §4）：
 * 1. 更新源为 MirrorChyan 且 CDK 已填写、MirrorChyan 给了 url → 用 MirrorChyan；
 * 2. 更新源为 GitHub，或 CDK 未填写（MirrorChyan 源回退）、或 MirrorChyan 未给 url
 *    → 从 GitHub 匹配 tag 与资产（含 digest）。错误码场景已在 `checkUpdate` 抛错，不会走到这里。
 */
async function prepareDownload(): Promise<PreparedUpdate | null> {
  const state = updateCheckResult.value;
  if (state.status !== UpdateCheckStatus.HasUpdate || !state.result.data) {
    return null;
  }
  const data = state.result.data;
  const cdk = mirrorchyanCdk.value.trim();
  const versionName = data.version_name;

  // 尊重「下载源」设置：只有选择 MirrorChyan 且 CDK 已填写时才走镜像；
  // 其余情况（源为 GitHub / CDK 未填写 / MirrorChyan 未给 url）一律回退 GitHub。
  if (oeaConfig.value.updateSource === UpdateSource.Mirrorchyan && cdk && data.url) {
    return {
      url: data.url,
      sha256: data.sha256,
      fileSize: data.filesize,
      filename: extractFilenameFromUrl(data.url),
      source: UpdateSource.Mirrorchyan,
      updateType:
        data.update_type === 'incremental' ? UpdatePackageType.Incremental : UpdatePackageType.Full,
      versionName,
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
 * 从 GitHub Releases 获取目标版本的下载资产。
 *
 * - 按已知 tag（`v<新版本>`）直接请求「按 tag 获取 release」端点，
 *   避免列表接口只返回前 100 条导致的分页遗漏；
 * - 匹配资产：优先精确匹配 `OEA-<平台>-<架构>-v<版本>.zip`，无精确匹配时取体积最大的 zip 资产；
 * - 校验信息：取 asset `digest`（须为 `sha256:<hex>` 格式，否则视为无校验信息）；
 * - 下载地址：使用 asset 的 API `url`（`/releases/assets/{id}`）而非 `browser_download_url`，
 *   后者在 private 仓库中不可用；Rust 下载端会带 `Authorization` 与
 *   `Accept: application/octet-stream` 请求并跟随 302 重定向。
 */
async function resolveGithubDownload(
  versionName: string,
): Promise<{ url: string; sha256?: string; fileSize: number; filename: string } | null> {
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': buildUpdateUserAgent(),
  };
  if (__OEA_GITHUB_TOKEN__) {
    headers.Authorization = `Bearer ${__OEA_GITHUB_TOKEN__}`;
  }

  const init: RequestInit & ClientOptions = {
    method: 'GET',
    headers,
    ...(await buildProxyClientOptions()),
  };

  const target = normalizeVersion(versionName);
  const tag = `v${target}`;
  let response: Response;
  try {
    response = await fetch(`${GITHUB_RELEASES_URL}/tags/${encodeURIComponent(tag)}`, init);
  } catch (error) {
    throw new Error(
      `GitHub API 请求失败: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (response.status === 404) {
    throw new Error(`GitHub 上未找到版本 ${tag} 的 Release`);
  }
  if (!response.ok) {
    throw new Error(`GitHub API 错误（HTTP ${response.status}），GitHub 下载暂不可用`);
  }

  const release: GitHubRelease = await response.json();

  // 匹配资产：优先精确匹配 `OEA-<平台>-<架构>-v<版本>.zip`，无精确匹配时取体积最大的 zip 资产。
  const exactName = `OEA-${platform()}-${arch()}-${tag}.zip`;
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

/** 从 URL 路径提取文件名（带扩展名才返回）。 */
function extractFilenameFromUrl(url: string): string | undefined {
  try {
    const segment = new URL(url).pathname.split('/').filter(Boolean).pop();
    if (!segment) {
      return undefined;
    }
    const filename = decodeURIComponent(segment);
    return filename.includes('.') ? filename : undefined;
  } catch {
    return undefined;
  }
}

/** 归一化版本号（去 `v` 前缀、小写）。 */
function normalizeVersion(version: string): string {
  return version.replace(/^v/i, '').toLowerCase();
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
