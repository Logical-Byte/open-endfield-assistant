import { cdkDecrypt, cdkEncrypt, loadOeaConfig, saveOeaConfig } from '@/utils/tauri';

import { UpdateProxyMode, UpdateSource, type SettingsDraft } from './model';

/** 与 ScanGuide 内容同步的持久化提示版本。修改提示文案时在这里决定是否递增。 */
export const CURRENT_SCAN_TIPS_VERSION: number = 1;
const CURRENT_MAJOR_VERSION: number = 0;
const CURRENT_MINOR_VERSION: number = 0;

/**
 * `load_oea_config` / `save_oea_config` 的完整持久化格式。
 *
 * 它是 settings persistence adapter 的内部实现细节，逻辑设置调用者不会接触版本、
 * CDK 密文或扫描提示版本字段。
 */
export interface PersistedOeaConfig {
  majorVersion: number;
  minorVersion: number;
  minimizeToTray: boolean;
  soundVolume: number;
  updateSource: UpdateSource;
  mirrorchyanCdkEncrypted: string;
  updateProxyMode: UpdateProxyMode;
  updateProxyUrl: string;
  autoDownloadUpdates: boolean;
  autoInstallUpdates: boolean;
  scanTipsDismissedVersion: number;
}

export const DEFAULT_OEA_CONFIG: PersistedOeaConfig = {
  majorVersion: CURRENT_MAJOR_VERSION,
  minorVersion: CURRENT_MINOR_VERSION,
  minimizeToTray: false,
  soundVolume: 0.5,
  updateSource: UpdateSource.Mirrorchyan,
  mirrorchyanCdkEncrypted: '',
  updateProxyMode: UpdateProxyMode.System,
  updateProxyUrl: '',
  autoDownloadUpdates: true,
  autoInstallUpdates: true,
  scanTipsDismissedVersion: 0,
};

/** Settings 内核与实际存储之间的最小接口。 */
export interface SettingsPersistence {
  load(): Promise<PersistedOeaConfig>;
  save(candidate: Readonly<PersistedOeaConfig>): Promise<void>;
  encryptCdk(plain: string): Promise<string>;
  decryptCdk(encrypted: string): Promise<string>;
}

export function createTauriSettingsPersistence(): SettingsPersistence {
  return {
    load: loadOeaConfig<PersistedOeaConfig>,
    save: saveOeaConfig,
    encryptCdk: cdkEncrypt,
    decryptCdk: cdkDecrypt,
  };
}

export function createDefaultSettingsDraft(): SettingsDraft {
  return settingsDraftFromPersisted(DEFAULT_OEA_CONFIG, '');
}

export function settingsDraftFromPersisted(
  config: PersistedOeaConfig,
  mirrorchyanCdk: string,
): SettingsDraft {
  return {
    minimizeToTray: config.minimizeToTray,
    soundVolume: config.soundVolume,
    updateSource: config.updateSource,
    mirrorchyanCdk,
    updateProxyMode: config.updateProxyMode,
    updateProxyUrl: config.updateProxyUrl,
    autoDownloadUpdates: config.autoDownloadUpdates,
    autoInstallUpdates: config.autoInstallUpdates,
    scanGuideEnabled: config.scanTipsDismissedVersion < CURRENT_SCAN_TIPS_VERSION,
  };
}

export function persistedFromSettingsDraft(
  candidate: Readonly<SettingsDraft>,
  baseline: Readonly<PersistedOeaConfig>,
  encryptedCdk: string,
): PersistedOeaConfig {
  return {
    ...baseline,
    minimizeToTray: candidate.minimizeToTray,
    soundVolume: candidate.soundVolume,
    updateSource: candidate.updateSource,
    mirrorchyanCdkEncrypted: encryptedCdk,
    updateProxyMode: candidate.updateProxyMode,
    updateProxyUrl: candidate.updateProxyUrl,
    autoDownloadUpdates: candidate.autoDownloadUpdates,
    autoInstallUpdates: candidate.autoInstallUpdates,
    scanTipsDismissedVersion: candidate.scanGuideEnabled ? 0 : CURRENT_SCAN_TIPS_VERSION,
  };
}
