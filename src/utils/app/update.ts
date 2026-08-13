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

    const response = await fetch(url, init);
    const payload = await response.json();
    maybePayload = payload;

    if (payload.code !== 0 || !payload.data) {
      throw new Error(`${payload.msg} (${payload.code})`);
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

/** 判断 latest 是否比 current 更新；任一侧无法解析时保守视为无更新。 */
function isNewer(latest: string, current: string): boolean {
  try {
    return gt(latest, current);
  } catch (error) {
    logError(`检查更新：版本号比较失败，视为无更新: ${JSON.stringify({ latest, current, error })}`);
    return false;
  }
}
