import { MirrorchyanResourcesLatestResponse } from '@/types/mirrorchyan';
import { UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import {
  PreparedUpdate,
  UpdateCheckResult,
  UpdateCheckStatus,
  UpdateDownloadProgress,
  UpdateDownloadStatus,
} from '@/types/update';
import { appVersion } from '@/utils/app/appVersion';
import { mirrorchyanCdk, oeaConfig } from '@/utils/app/config';
import { logError, logInfo, logWarn } from '@/utils/tauri';
import { updatePopoverOpen } from '@/utils/uiState';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { fetch, type ClientOptions } from '@tauri-apps/plugin-http';
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

/** 当前下载会话编号（过滤旧任务的迟到进度事件）。 */
let currentSessionId: number | null = null;
/** 下载互斥：同一时间只允许一个下载任务。 */
let isDownloading = false;
/** 是否为用户主动取消（取消不视为错误）。 */
let downloadCancelled = false;

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

/** GitHub Release API 的最小响应结构（本地 schema 副本确认含 `digest`）。 */
interface GitHubRelease {
  tag_name: string;
  assets: GitHubReleaseAsset[];
}

interface GitHubReleaseAsset {
  name: string;
  size: number;
  /** GitHub API 资产端点；private 仓库下载必须用它（配 `Accept: application/octet-stream`）。 */
  url: string;
  digest?: string;
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
    // 读取当前版本号，若无法获取则直接报错。
    const currentVersion = appVersion.value;
    if (currentVersion === null) {
      throw new Error('无法获取当前版本号');
    }

    // 构造请求参数（主备站共用同一套参数）。
    const url = new URL(CHECK_URL_BASES[0]);
    url.searchParams.set('current_version', `v${currentVersion}`);
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
      'User-Agent': `OEA/${currentVersion} (Windows NT 10.0; Win64; x64; amd64)`,
      Accept: 'application/json',
    };

    // 解析检查请求使用的代理（System 模式从注册表读取，Custom 用配置 URL）。
    const proxyInit = await buildProxyClientOptions();

    // 依次尝试主站与备站。
    let payload: MirrorchyanResourcesLatestResponse | undefined = undefined;
    let lastError: unknown = undefined;
    for (const base of CHECK_URL_BASES) {
      try {
        const response = await fetch(`${base}?${params}`, {
          method: 'GET',
          headers,
          ...proxyInit,
        });
        const parsed = (await response.json()) as MirrorchyanResourcesLatestResponse;
        payload = parsed;
        if (parsed.code === 0) {
          break;
        }
        lastError = new Error(`Mirror 酱服务返回错误: code=${parsed.code}, msg=${parsed.msg}`);
        logWarn(`${base} 返回错误 code=${parsed.code}，尝试备用站`);
      } catch (error) {
        lastError = error;
        logWarn(`${base} 请求失败: ${error instanceof Error ? error.message : String(error)}`);
      }
    }

    if (!payload) {
      const errorMessage = lastError instanceof Error ? lastError.message : String(lastError);
      throw new Error(`检查更新请求失败，请检查网络连接或代理设置，或稍后重试。\n${errorMessage}`);
    }
    maybePayload = payload;

    // 检查业务错误码，若非 0 则视为失败。
    if (payload.code !== 0) {
      throw buildMirrorchyanError(payload.code, payload.msg);
    }
    if (!payload.data) {
      throw new Error('检查更新服务响应异常，请稍后重试');
    }

    const data = payload.data;
    const latestVersion = data.version_name;
    const hasUpdate = isNewer(latestVersion, currentVersion);

    if (hasUpdate) {
      logWarn(`检查更新：有新版本可用，当前 v${currentVersion}，最新 ${latestVersion}`);
      updateCheckResult.value = { status: UpdateCheckStatus.HasUpdate, result: payload };
      updatePopoverOpen.value = true;
      // 「自动下载更新」开启时直接开始下载（无需用户点击）。
      if (oeaConfig.value.autoDownloadUpdates) {
        void startDownload();
      }
    } else {
      logInfo(`检查更新：已是最新版本 v${currentVersion}`);
      updateCheckResult.value = { status: UpdateCheckStatus.NoUpdate, result: payload };
    }
  } catch (error) {
    const errorInstance = error instanceof Error ? error : new Error(String(error));
    updateCheckResult.value = {
      status: UpdateCheckStatus.Error,
      error: errorInstance,
      result: maybePayload,
    };
    updatePopoverOpen.value = true;
    logError(`检查更新失败: ${errorInstance.message}`);
  }
}

/**
 * 开始下载更新（自动下载与手动「立即更新」共用）。
 *
 * 流程：准备下载信息（决定源与校验信息）→ 调 Rust `download_update` 流式下载 →
 * 监听 `download-progress` 进度事件（按 session 过滤）→ 成功后保存实际路径。
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
    const proxyMode = prepared.source === 'github' ? updateProxyMode : UpdateProxyMode.None;
    const result = await invoke<DownloadResultPayload>('download_update', {
      url: prepared.url,
      savePath,
      totalSize: prepared.fileSize ?? null,
      expectedSha256: prepared.sha256 ?? null,
      proxyMode,
      proxyUrl: proxyMode === UpdateProxyMode.Custom ? updateProxyUrl : null,
      authToken: prepared.source === 'github' ? __OEA_GITHUB_TOKEN__ : null,
      accept: prepared.source === 'github' ? 'application/octet-stream' : null,
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
    // 阶段 3 接入：保存 pending 信息 + 按「自动安装更新」触发安装。
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
      source: 'mirrorchyan',
      updateType:
        data.update_type === 'incremental' || data.update_type === 'full'
          ? data.update_type
          : undefined,
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
    source: 'github',
    versionName,
    releaseNote: data.release_note,
  };
}

/**
 * 从 GitHub Releases 匹配目标版本的下载资产。
 *
 * - 匹配 tag：`v<新版本>`（归一化后比较），找不到则回退最新 release（v4 §12 风险注记）；
 * - 匹配资产：`OEA-windows-x86_64-v<版本>.zip`；
 * - 校验信息：取 asset `digest`（`sha256:<hex>`，去前缀）；
 * - 下载地址：使用 asset 的 API `url`（`/releases/assets/{id}`）而非 `browser_download_url`，
 *   后者在 private 仓库中不可用；Rust 下载端会带 `Authorization` 与
 *   `Accept: application/octet-stream` 请求并跟随 302 重定向。
 */
async function resolveGithubDownload(
  versionName: string,
): Promise<{ url: string; sha256?: string; fileSize: number; filename: string } | null> {
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': `OEA/${appVersion.value ?? 'unknown'}`,
  };
  if (__OEA_GITHUB_TOKEN__) {
    headers.Authorization = `token ${__OEA_GITHUB_TOKEN__}`;
  }

  const init: RequestInit & ClientOptions = {
    method: 'GET',
    headers,
    ...(await buildProxyClientOptions()),
  };

  let response: Response;
  try {
    response = await fetch(`${GITHUB_RELEASES_URL}?per_page=100`, init);
  } catch (error) {
    throw new Error(
      `GitHub API 请求失败: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (!response.ok) {
    throw new Error(`GitHub API 错误（HTTP ${response.status}），GitHub 下载暂不可用`);
  }

  const releases = (await response.json()) as GitHubRelease[];

  const target = normalizeVersion(versionName);
  const release =
    releases.find((item) => normalizeVersion(item.tag_name) === target) ?? releases[0] ?? null;
  if (!release) {
    throw new Error('GitHub 上未找到对应版本的 Release');
  }

  const candidates = release.assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    return name.includes('windows') && name.includes('x86_64') && name.endsWith('.zip');
  });
  const exactName = `OEA-windows-x86_64-v${target}.zip`;
  const asset =
    candidates.find((item) => item.name === exactName) ??
    candidates.find((item) =>
      item.name.toLowerCase().includes(`v${normalizeVersion(versionName)}`),
    ) ??
    candidates[0] ??
    null;
  if (!asset) {
    throw new Error('GitHub Release 中未找到 OEA-windows-x86_64 的 zip 资产');
  }

  const sha256 = asset.digest?.replace(/^sha256:/i, '').trim();
  if (!sha256) {
    logWarn(`GitHub 资产缺少 digest，跳过 sha256 校验: ${asset.name}`);
  }
  return {
    url: asset.url,
    sha256: sha256 || undefined,
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
