import { UpdateProxyMode, UpdateSource, type OeaConfig } from '@/types/oeaConfig';
import { cdkDecrypt, cdkEncrypt, loadOeaConfig, saveOeaConfig } from '@/utils/tauri';

import type { SettingsDraft } from './model';

export const CURRENT_SCAN_TIPS_VERSION: number = 1;

export const DEFAULT_OEA_CONFIG: OeaConfig = {
  majorVersion: 0,
  minorVersion: 0,
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
  load(): Promise<OeaConfig>;
  save(candidate: Readonly<OeaConfig>): Promise<void>;
  encryptCdk(plain: string): Promise<string>;
  decryptCdk(encrypted: string): Promise<string>;
}

export function createTauriSettingsPersistence(): SettingsPersistence {
  return {
    load: loadOeaConfig,
    save: saveOeaConfig,
    encryptCdk: cdkEncrypt,
    decryptCdk: cdkDecrypt,
  };
}

export function createDefaultSettingsDraft(): SettingsDraft {
  return settingsDraftFromPersisted(DEFAULT_OEA_CONFIG, '');
}

export function settingsDraftFromPersisted(
  config: OeaConfig,
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
  baseline: Readonly<OeaConfig>,
  encryptedCdk: string,
): OeaConfig {
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
