import type { DeepReadonly } from 'vue';

export enum UpdateSource {
  Mirrorchyan = 'mirrorchyan',
  Oem = 'oem',
  Github = 'github',
}

export enum UpdateProxyMode {
  None = 'none',
  System = 'system',
  Custom = 'custom',
}

/** 更新设置页唯一维护的更新源展示项。 */
export const updateSourceItems = [
  { label: 'Mirror酱', value: UpdateSource.Mirrorchyan },
  { label: 'OEM', value: UpdateSource.Oem },
  { label: 'GitHub', value: UpdateSource.Github },
];

/** 更新设置页唯一维护的下载代理展示项。 */
export const proxyModeItems = [
  { label: '不使用代理', value: UpdateProxyMode.None },
  { label: '系统代理', value: UpdateProxyMode.System },
  { label: '自定义代理', value: UpdateProxyMode.Custom },
];

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
  | { kind: 'decrypt-error'; error: unknown }
  | { kind: 'save-error'; error: unknown };
