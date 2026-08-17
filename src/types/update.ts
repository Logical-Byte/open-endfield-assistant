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

export interface UpdateSnapshot {
  status: UpdateStatus;
  currentVersion: string;
  availableVersion: string | null;
  releaseNotes: string | null;
  downloadedBytes: number;
  totalBytes: number | null;
  bytesPerSecond: number;
  error: string | null;
}
