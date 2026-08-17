import type { UpdateSnapshot, UpdateStatus } from '@/types/update';

const INSTALL_STATUSES = new Set<UpdateStatus>([
  'downloading',
  'verifying',
  'preparing',
  'bootstrapReady',
]);

const STAGE_LABELS: Partial<Record<UpdateStatus, string>> = {
  downloading: '正在下载完整更新包',
  verifying: '正在校验 SHA-256',
  preparing: '正在准备新版本资源',
  bootstrapReady: '准备完成，即将重启',
  failed: '更新失败',
};

/** 返回该状态是否属于不可关闭的安装准备过程。 */
export function isUpdateInstalling(status: UpdateStatus): boolean {
  return INSTALL_STATUSES.has(status);
}

/** 返回工具栏是否应显示更新入口。 */
export function isUpdatePopoverVisible(status: UpdateStatus): boolean {
  return status !== 'idle' && status !== 'upToDate';
}

/** 返回状态是否需要用提示点主动吸引用户注意。 */
export function needsUpdateAttention(status: UpdateStatus): boolean {
  return status === 'available' || status === 'failed';
}

/** 把 Rust 状态名翻译为安装弹窗中的阶段说明。 */
export function updateStageLabel(status: UpdateStatus): string {
  return STAGE_LABELS[status] ?? '正在准备';
}

/** 计算确定进度条百分比；总大小未知时返回 `null`。 */
export function downloadPercentage(snapshot: UpdateSnapshot): number | null {
  const total = snapshot.totalBytes;
  return total && total > 0 ? (snapshot.downloadedBytes / total) * 100 : null;
}

/** 将字节数格式化为紧凑的人类可读单位；展示方式改编自 PR #6。 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;
  return `${value.toFixed(value >= 100 || index === 0 ? 0 : 1)} ${units[index]}`;
}

/** 生成“已下载 / 总大小”文本；展示方式改编自 PR #6。 */
export function downloadProgressText(snapshot: UpdateSnapshot): string {
  const downloaded = formatBytes(snapshot.downloadedBytes);
  return snapshot.totalBytes === null
    ? downloaded
    : `${downloaded} / ${formatBytes(snapshot.totalBytes)}`;
}

/** 按剩余字节和当前速度估算时间；展示方式改编自 PR #6。 */
export function downloadEtaText(snapshot: UpdateSnapshot): string | null {
  const { bytesPerSecond, downloadedBytes, totalBytes } = snapshot;
  if (!totalBytes || bytesPerSecond <= 0 || downloadedBytes >= totalBytes) return null;
  const seconds = Math.ceil((totalBytes - downloadedBytes) / bytesPerSecond);
  return seconds < 60 ? `约 ${seconds} 秒` : `约 ${Math.ceil(seconds / 60)} 分钟`;
}
