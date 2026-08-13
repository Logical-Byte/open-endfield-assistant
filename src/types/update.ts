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
