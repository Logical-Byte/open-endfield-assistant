import type { UpdateSnapshot } from '@/types/update';
import { initOeaConfig, oeaConfig } from '@/utils/app/config';
import { isUpdateInstalling, needsUpdateAttention } from '@/utils/update-display';
import {
  checkForUpdate,
  downloadAndInstallUpdate,
  getUpdateSnapshot,
  onUpdateState,
} from '@/utils/tauri';
import { computed, ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';

// Vue 只保存并展示 Rust 快照；下载、校验、解压和文件替换都由 Rust 编排。
export const updateSnapshot = ref<UpdateSnapshot>({
  status: 'idle',
  currentVersion: '0.0.0',
  availableVersion: null,
  releaseNotes: null,
  downloadedBytes: 0,
  totalBytes: null,
  bytesPerSecond: 0,
  error: null,
});
export const updatePopoverOpen = ref(false);
export const installUpdateModalOpen = ref(false);

// 首次显式更新操作创建此 Promise；后续操作等待同一次监听器和快照初始化。
let initialization: Promise<void> | null = null;
// 记录可用版本元数据对应的来源设置，防止设置变更后继续安装旧 URL。
const checkedMetadataKey = ref<string | null>(null);

/** 把会影响下载地址的配置压缩成可比较的来源标识。 */
function metadataKey(): string {
  const { updateSource, mirrorchyanCdk } = oeaConfig.value;
  // 代理只影响传输路径，不改变已经检查得到的包身份，因此无需让元数据过期。
  return updateSource === 'mirrorchyan' && mirrorchyanCdk.trim()
    ? `mirrorchyan:${mirrorchyanCdk.trim()}`
    : 'github';
}

/** 当前可用版本是否由旧的来源/CDK 配置检查得到。 */
export const updateMetadataStale = computed(
  () =>
    updateSnapshot.value.availableVersion !== null &&
    checkedMetadataKey.value !== null &&
    checkedMetadataKey.value !== metadataKey(),
);

/** 用 Rust 的完整快照替换本地状态，并同步需要用户注意的界面。 */
function applySnapshot(snapshot: UpdateSnapshot): void {
  updateSnapshot.value = snapshot;
  if (snapshot.status === 'available') {
    checkedMetadataKey.value = metadataKey();
  } else if (snapshot.availableVersion === null) {
    checkedMetadataKey.value = null;
  }
  if (needsUpdateAttention(snapshot.status)) {
    updatePopoverOpen.value = true;
  }
  if (isUpdateInstalling(snapshot.status)) {
    installUpdateModalOpen.value = true;
  }
}

/** 在首次显式更新操作前注册 Rust 事件并同步当前快照。 */
async function ensureUpdateState(): Promise<void> {
  if (!isTauri()) return;
  const pending = (initialization ??= (async () => {
    await initOeaConfig();
    const unlisten = await onUpdateState(applySnapshot);
    try {
      applySnapshot(await getUpdateSnapshot());
    } catch (error) {
      unlisten();
      throw error;
    }
  })());
  try {
    await pending;
  } catch (error) {
    // 不缓存失败结果，让下一次用户操作重新注册监听器并读取快照。
    if (initialization === pending) initialization = null;
    throw error;
  }
}

/** 初始化更新事件，并按配置在后台执行只获取元数据的启动检查。 */
export async function initUpdateState(): Promise<void> {
  try {
    await ensureUpdateState();
    if (oeaConfig.value.checkUpdates) {
      await checkUpdate();
    }
  } catch (error) {
    console.error('初始化更新状态失败', error);
  }
}

/** 请求 Rust 获取更新元数据；失败详情仍以 Rust 发出的快照为准。 */
export async function checkUpdate(): Promise<void> {
  await ensureUpdateState();
  try {
    await checkForUpdate();
    applySnapshot(await getUpdateSnapshot());
  } catch {
    // Rust emits the authoritative Failed snapshot.
  }
}

/** 请求 Rust 完成下载和准备；前端只负责切换到安装进度界面。 */
export async function downloadAndInstall(): Promise<void> {
  await ensureUpdateState();
  installUpdateModalOpen.value = true;
  updatePopoverOpen.value = false;
  try {
    await downloadAndInstallUpdate();
  } catch {
    // Rust emits preparation failures. Platform launch errors are returned to the caller.
  }
}
