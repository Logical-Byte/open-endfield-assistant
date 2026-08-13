import { MirrorchyanResourcesLatestResponse } from '@/types/mirrorchyan';
import { UpdateProxyMode } from '@/types/oeaConfig';
import { UpdateCheckResult, UpdateCheckStatus } from '@/types/update';
import { appVersion } from '@/utils/app/appVersion';
import { mirrorchyanCdk, oeaConfig } from '@/utils/app/config';
import { logError, logInfo, logWarn } from '@/utils/tauri';
import { updatePopoverOpen } from '@/utils/uiState';
import { fetch, type ClientOptions } from '@tauri-apps/plugin-http';
import { gt } from 'semver';
import { ref } from 'vue';

/** MirrorChyan 资源 ID（与发布侧 rid 一致，大写）。 */
const RESOURCE_ID = 'OEA';
/** 检查更新 API 端点。 */
const CHECK_URL = `https://mirrorchyan.com/api/resources/${RESOURCE_ID}/latest`;

export const updateCheckResult = ref<UpdateCheckResult>({ status: UpdateCheckStatus.Idle });

/**
 * 执行一次检查更新（启动自动检查与设置页手动检查共用）。
 *
 * 检查固定请求 MirrorChyan，无论「更新源」设置为何：更新源只决定后续下载
 * 从镜像还是 GitHub 拉取，不影响「是否可更新」的判定（见设计文档 v1）。
 */
export async function checkUpdate(): Promise<void> {
  updateCheckResult.value = { status: UpdateCheckStatus.Checking };
  let maybePayload: MirrorchyanResourcesLatestResponse | undefined = undefined;
  try {
    // 读取当前版本号，若无法获取则直接报错。
    const currentVersion = appVersion.value;
    if (currentVersion === null) {
      throw new Error('无法获取当前版本号');
    }

    // 构造请求 URL 与参数。
    const url = new URL(CHECK_URL);
    url.searchParams.set('current_version', `v${currentVersion}`);
    url.searchParams.set('user_agent', 'oea_client');
    url.searchParams.set('channel', 'stable');
    url.searchParams.set('os', 'windows');
    url.searchParams.set('arch', 'amd64');
    const cdk = mirrorchyanCdk.value.trim();
    if (cdk) {
      url.searchParams.set('cdk', cdk);
    }

    const init: RequestInit & ClientOptions = {
      method: 'GET',
      headers: {
        'User-Agent': `OEA/${currentVersion} (Windows NT 10.0; Win64; x64; amd64)`,
        Accept: 'application/json',
      },
    };

    // TODO: 当前未实现 “使用系统代理” 的功能。
    const { updateProxyMode, updateProxyUrl } = oeaConfig.value;
    if (updateProxyMode === UpdateProxyMode.Custom && updateProxyUrl) {
      init.proxy = { all: updateProxyUrl };
    }

    // 执行请求，捕获网络错误。
    let response: Response;
    try {
      response = await fetch(url, init);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new Error(`检查更新请求失败，请检查网络连接或代理设置，或稍后重试。\n${errorMessage}`);
    }

    // 解析响应 JSON，捕获解析错误。
    let payload: MirrorchyanResourcesLatestResponse;
    try {
      payload = await response.json();
    } catch {
      throw new Error(`检查更新服务异常（HTTP ${response.status}）`);
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
      logWarn(`检查更新：有新版本可用，当前 ${currentVersion}，最新 ${latestVersion}`);
      updateCheckResult.value = { status: UpdateCheckStatus.HasUpdate, result: payload };
      updatePopoverOpen.value = true;
    } else {
      logInfo(`检查更新：已是最新版本 ${currentVersion}`);
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

/** MirrorChyan 业务错误码对应的用户可读描述（见 `docs/ErrorCode.md`）。 */
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
