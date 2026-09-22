import type { DeepReadonly } from 'vue';

import { UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';

/** 用户可读写的逻辑设置，不包含持久化格式字段。 */
export interface SettingsDraft {
  minimizeToTray: boolean;
  soundVolume: number;
  updateSource: UpdateSource;
  mirrorchyanCdk: string;
  updateProxyMode: UpdateProxyMode;
  updateProxyUrl: string;
  autoDownloadUpdates: boolean;
  autoInstallUpdates: boolean;
  scanGuideEnabled: boolean;
}

/** 最近一次成功保存且已生效的逻辑设置快照。 */
export type SettingsSnapshot = DeepReadonly<SettingsDraft>;

export type SettingsStatus =
  | { kind: 'loading' }
  | { kind: 'idle' }
  | { kind: 'saving' }
  | { kind: 'load-error'; error: unknown }
  | { kind: 'save-error'; error: unknown };
