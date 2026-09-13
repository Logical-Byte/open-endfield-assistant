/** 后端检查到的可用更新。 */
export interface AvailableUpdate {
  versionName: string;
  releaseNote: string;
}

/** `check_update` 命令返回值。 */
export type UpdateAvailability =
  { status: 'upToDate' } | { status: 'available'; update: AvailableUpdate };

/** 当前 WebView 生命周期内的检查状态。 */
export type UpdateCheckState =
  | { status: 'unknown'; lastCheckedAt: null }
  | { status: 'checking'; lastCheckedAt: number | null }
  | { status: 'upToDate'; lastCheckedAt: number }
  | { status: 'available'; lastCheckedAt: number; update: AvailableUpdate }
  | { status: 'error'; lastCheckedAt: number | null; error: Error };

/** 下载进度（来自本次 `download_update` 调用独享的 IPC Channel）。 */
export interface DownloadProgress {
  /** 已下载字节数 */
  downloadedSize: number;
  /** 总字节数（未知为 0） */
  totalSize: number;
  /** 瞬时速度（字节/秒，EMA 平滑） */
  speed: number;
  /** 进度百分比 0~100 */
  progress: number;
}

/** 后端完成原子发布后的可安装更新。 */
export interface DownloadedUpdate {
  downloadedPackagePath: string;
  versionName: string;
  releaseNote: string;
}

/** 下载及其可安装结果的唯一状态。 */
export type DownloadState =
  | { status: 'idle' }
  | { status: 'downloading'; progress: DownloadProgress }
  | { status: 'cancelling'; progress: DownloadProgress }
  | { status: 'completed'; update: DownloadedUpdate }
  | { status: 'failed' };

/** 安装阶段。 */
export enum UpdateInstallStatus {
  Idle,
  Installing,
  Completed,
  Failed,
}

/** 安装流程阶段（驱动不可关闭弹窗的进度文案）。 */
export enum UpdateInstallStage {
  /** 构造 candidate 的准备阶段。 */
  Preparing = 'preparing',
  Extracting = 'extracting',
  ApplyingIncremental = 'applying_incremental',
  ApplyingFull = 'applying_full',
  CleaningUp = 'cleaning_up',
}

/** Rust `update-install-stage` 事件 payload。 */
export interface UpdateInstallStageEvent {
  stage: UpdateInstallStage;
}

/** 待安装更新信息（localStorage `oea-pending-update`，下载完成 → 安装完成之间持久化）。 */
export interface PendingUpdateInfo {
  /** 开始这次安装时运行中的版本，用于重启后的完成提示。 */
  previousVersion?: string;
  versionName: string;
  releaseNote: string;
  downloadSavePath: string;
  timestamp: number;
}

/** 更新完成弹窗使用的信息；跨进程 metadata 缺失时只保证 `timestamp`。 */
export interface UpdateCompleteInfo {
  previousVersion?: string;
  newVersion?: string;
  releaseNote?: string;
  timestamp: number;
}
