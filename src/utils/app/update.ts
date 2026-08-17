import type { UpdateSnapshot } from '@/types/update';
import { oeaConfig } from '@/utils/app/config';
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

// 事件监听只注册一次，避免页面重建时重复处理同一 Rust 事件。
let initialized = false;
// 记录可用版本元数据对应的来源设置，防止设置变更后继续安装旧 URL。
const checkedMetadataKey = ref<string | null>(null);

/** 把会影响下载地址的配置压缩成可比较的来源标识。 */
function metadataKey(): string {
  const { updateSource, mirrorchyanCdk } = oeaConfig.value;
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

/** 注册 Rust 状态事件、读取当前快照，并按配置执行一次启动检查。 */
export async function initUpdateState(): Promise<void> {
  if (initialized) return;
  initialized = true;
  if (!isTauri()) return;
  await onUpdateState(applySnapshot);
  applySnapshot(await getUpdateSnapshot());
  if (oeaConfig.value.checkUpdates) {
    await checkUpdate();
  }
}

/** 请求 Rust 获取更新元数据；失败详情仍以 Rust 发出的快照为准。 */
export async function checkUpdate(): Promise<void> {
  try {
    await checkForUpdate();
    applySnapshot(await getUpdateSnapshot());
  } catch {
    // Rust emits the authoritative Failed snapshot.
  }
}

/** 请求 Rust 完成下载和准备；前端只负责切换到安装进度界面。 */
export async function downloadAndInstall(): Promise<void> {
  installUpdateModalOpen.value = true;
  updatePopoverOpen.value = false;
  try {
    await downloadAndInstallUpdate();
  } catch {
    // Rust emits preparation failures. Platform launch errors are returned to the caller.
  }
}
