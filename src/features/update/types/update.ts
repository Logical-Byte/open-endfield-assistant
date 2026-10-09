import type { DownloadProgress, UpdateInfo } from '@/shared/types/update';

export type {
  DownloadProgress,
  InstallStage as UpdateInstallStage,
  InstallStageEvent as UpdateInstallStageEvent,
  UpdateAvailability,
  UpdateInfo,
  UpdateOperationStatus as UpdateOperation,
  UpdateStatus,
} from '@/shared/types/update';

/** 当前 WebView 生命周期内的检查状态。 */
export type UpdateCheckState =
  | { status: 'unknown'; lastCheckedAt: null }
  | { status: 'checking'; lastCheckedAt: number | null }
  | { status: 'upToDate'; lastCheckedAt: number }
  | { status: 'available'; lastCheckedAt: number | null; update: UpdateInfo }
  | { status: 'error'; lastCheckedAt: number | null; error: Error };

/** 由后端 operation/pending 与 WebView 展示状态推导的下载状态。 */
export type DownloadState =
  | { status: 'idle' }
  | { status: 'downloading'; progress: DownloadProgress }
  | { status: 'cancelling'; progress: DownloadProgress }
  | { status: 'completed'; update: UpdateInfo }
  | { status: 'failed' };

/** 安装阶段。 */
export enum UpdateInstallStatus {
  Idle,
  Installing,
  Completed,
  Failed,
}

/** 更新完成弹窗使用的信息；跨进程 metadata 缺失时只保证 `timestamp`。 */
export interface UpdateCompleteInfo {
  previousVersion?: string;
  newVersion?: string;
  releaseNote?: string;
  timestamp: number;
}
