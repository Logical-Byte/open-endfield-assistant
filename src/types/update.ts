import { MirrorchyanResourcesLatestResponse } from '@/types/mirrorchyan';

/** 检查更新阶段。 */
export enum UpdateCheckStatus {
  Idle,
  Checking,
  HasUpdate,
  NoUpdate,
  Error,
}

export type UpdateCheckResult =
  | { status: UpdateCheckStatus.Idle }
  | { status: UpdateCheckStatus.Checking }
  | { status: UpdateCheckStatus.HasUpdate; result: MirrorchyanResourcesLatestResponse }
  | { status: UpdateCheckStatus.NoUpdate; result: MirrorchyanResourcesLatestResponse }
  | { status: UpdateCheckStatus.Error; error: Error; result?: MirrorchyanResourcesLatestResponse };

/** 下载阶段。 */
export enum UpdateDownloadStatus {
  Idle,
  Downloading,
  /** 已请求取消、Rust 仍在收尾（此时 `isDownloading` 仍为 true，不可开始新下载）。 */
  Cancelling,
  Completed,
  Failed,
}

/** 下载进度（来自 Rust `download-progress` 事件，字段已 camelCase）。 */
export interface UpdateDownloadProgress {
  /** 已下载字节数 */
  downloadedSize: number;
  /** 总字节数（未知为 0） */
  totalSize: number;
  /** 瞬时速度（字节/秒，EMA 平滑） */
  speed: number;
  /** 进度百分比 0~100 */
  progress: number;
}

/** 更新包下载源。 */
export type UpdateDownloadSource = 'mirrorchyan' | 'github';

/** 已就绪的下载信息（URL + 校验信息，由「下载源决策」产出）。 */
export interface PreparedUpdate {
  url: string;
  /** 期望 sha256（GitHub 来源时来自 asset.digest；缺失则为 undefined） */
  sha256?: string;
  fileSize?: number;
  /** 建议文件名（服务端可能通过 Content-Disposition 覆盖实际文件名） */
  filename?: string;
  source: UpdateDownloadSource;
  /** 更新包类型（仅 MirrorChyan 返回；GitHub 全量） */
  updateType?: 'incremental' | 'full';
  versionName: string;
  releaseNote: string;
}

/** 安装阶段。 */
export enum UpdateInstallStatus {
  Idle,
  Installing,
  Completed,
  Failed,
}

/** 安装流程阶段（驱动不可关闭弹窗的进度文案）。 */
export enum UpdateInstallStage {
  BackingUp,
  Extracting,
  Checking,
  ApplyingIncremental,
  ApplyingFull,
  CleaningUp,
  Done,
}

/** Mirror 酱增量包 `changes.json`（字段名 snake_case，与文档一致）。 */
export interface ChangesJson {
  added: string[];
  modified: string[];
  deleted: string[];
  added_dir: string[];
  deleted_dir: string[];
}

/** 待安装更新信息（localStorage `oea-pending-update`，下载完成 → 安装完成之间持久化）。 */
export interface PendingUpdateInfo {
  versionName: string;
  releaseNote: string;
  downloadSavePath: string;
  fileSize?: number;
  updateType?: 'incremental' | 'full';
  downloadSource?: UpdateDownloadSource;
  timestamp: number;
}

/** 更新完成信息（localStorage `oea-update-complete`，重启后展示）。 */
export interface UpdateCompleteInfo {
  previousVersion: string;
  newVersion: string;
  releaseNote: string;
  timestamp: number;
}
