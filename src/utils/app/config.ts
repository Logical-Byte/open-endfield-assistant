import { OeaConfig, UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import { cdkDecrypt, cdkEncrypt, loadOeaConfig, saveOeaConfig } from '@/utils/tauri';
import { createConfigStore } from './configStore';

/** 更新源选项 */
export const updateSourceItems = [
  { label: 'Mirror酱', value: UpdateSource.Mirrorchyan },
  { label: 'OEM', value: UpdateSource.Oem },
  { label: 'GitHub', value: UpdateSource.Github },
];

/** 更新代理模式选项 */
export const proxyModeItems = [
  { label: '不使用代理', value: UpdateProxyMode.None },
  { label: '系统代理', value: UpdateProxyMode.System },
  { label: '自定义代理', value: UpdateProxyMode.Custom },
];

export const CURRENT_MAJOR_VERSION: number = 0 as const;
export const CURRENT_MINOR_VERSION: number = 0 as const;

/**
 * 当前档案扫描提示的版本号（与 `ScanGuide.vue` 中的提示文案同源）。
 *
 * 展示规则：`oeaConfig.scanTipsDismissedVersion < CURRENT_SCAN_TIPS_VERSION` 时展示提示；
 * 用户勾选「下次更新前不再提示」并点击「我知道了」后，`scanTipsDismissedVersion`
 * 会被写入本值并持久化到后端配置（`config/oea_config.json`），之后本版本内不再展示。
 *
 * ▍让所有用户（含已确认过的）重新看一次新版提示：
 *   修改 `ScanGuide.vue` 的提示文案，同时把本常量递增 1（例如 1 → 2）。
 *   老用户已确认的版本号（1）将小于新版本（2），下次启动会重新看到新提示；
 *   从未确认过的新用户（0 < 2）同样会看到。
 *   这只改了前端常量与文案，不涉及 config 结构，因此无需 bump `minorVersion`。
 *
 * ▍更新提示文案但无需老用户重看：
 *   只修改 `ScanGuide.vue` 的提示文案，保持本常量不变。已确认过的用户不会重新看到；
 *   只有从未确认过的新用户会看到最新文案。
 */
export const CURRENT_SCAN_TIPS_VERSION: number = 1 as const;

export const DEFAULT_OEA_CONFIG: OeaConfig = {
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
} as const;

const settings = createConfigStore(DEFAULT_OEA_CONFIG, {
  load: loadOeaConfig,
  save: saveOeaConfig,
  encrypt: cdkEncrypt,
  decrypt: cdkDecrypt,
});

/** UI 读取 draft，业务流程读取 effective；修改统一经过 editSettings。 */
export const settingsDraft = settings.draft;
export const effectiveSettings = settings.effective;
export const configLoaded = settings.loaded;
export const configLoading = settings.loading;
export const configLoadError = settings.loadError;
export const configSaveError = settings.saveError;
export const initOeaConfig = settings.initialize;
export const editSettings = settings.edit;
export const retrySettingsSave = settings.retry;

/** Nuxt UI 滑块可能发出数组中间值，只接收合法音量。 */
export function setSoundVolume(value: number | number[] | undefined): void {
  if (typeof value === 'number' && Number.isFinite(value)) {
    settings.edit({ soundVolume: Math.min(1, Math.max(0, value)) });
  }
}
