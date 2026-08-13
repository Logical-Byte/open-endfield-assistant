import { MirrorchyanResourcesLatestResponse } from '@/types/mirrorchyan';

/** 检查更新阶段。 */
export enum UpdateCheckStatus {
  Idle,
  Checking,
  HasUpdate,
  NoUpdate,
  Error,
}

/** 检查更新结果。 */
export interface UpdateCheckResult {
  /** 是否有新版本可更新。 */
  hasUpdate: boolean;
  /** 当前版本号（如 "0.1.0"）。 */
  currentVersion: string;
  /** 最新版本号（如 "0.2.0"）。 */
  latestVersion: string;
  /** API 响应数据。 */
  payload: MirrorchyanResourcesLatestResponse;
}
