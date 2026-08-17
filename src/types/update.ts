/** Rust 更新状态机公开给 Vue 的阶段名称。 */
export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'downloading'
  | 'verifying'
  | 'preparing'
  | 'bootstrapReady'
  | 'failed';

/**
 * Rust 更新管理器发出的完整状态快照。
 *
 * 前端直接替换整份快照，不自行推断下载或文件事务状态。
 */
export interface UpdateSnapshot {
  /** 当前状态机阶段。 */
  status: UpdateStatus;
  /** 当前运行程序的内置版本。 */
  currentVersion: string;
  /** 有更新时的目标版本，尚未发现更新时为 `null`。 */
  availableVersion: string | null;
  /** 来源提供的 Markdown 更新日志。 */
  releaseNotes: string | null;
  /** 当前部分文件已经落盘的总字节数。 */
  downloadedBytes: number;
  /** 服务端未提供总大小时为 `null`。 */
  totalBytes: number | null;
  /** 本次运行新传输字节的平均速度。 */
  bytesPerSecond: number;
  /** 失败时由 Rust 格式化的错误链。 */
  error: string | null;
}
