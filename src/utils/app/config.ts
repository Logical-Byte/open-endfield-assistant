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

/** `DEFAULT_OEA_CONFIG.majorVersion` 使用的配置格式主版本。 */
export const CURRENT_MAJOR_VERSION: number = 0 as const;
/** `DEFAULT_OEA_CONFIG.minorVersion` 使用的配置格式次版本。 */
export const CURRENT_MINOR_VERSION: number = 0 as const;

/**
 * 当前档案扫描提示的版本号（与 `ScanGuide.vue` 中的提示文案同源）。
 *
 * 展示规则：`configInitialized.value === true` 且
 * `effectiveSettings.value.scanTipsDismissedVersion < CURRENT_SCAN_TIPS_VERSION` 时展示提示。
 * 用户勾选「下次更新前不再提示」并点击「我知道了」后，`scanTipsDismissedVersion`
 * 会被写入本值并持久化到后端配置（`config/oea_config.json`），之后本版本内不再展示。
 *
 * ▍让所有用户（含已确认过的）重新看一次新版提示：
 *   修改 `ScanGuide.vue` 的提示文案，同时把本常量递增 1（例如 1 → 2）。
 *   老用户已确认的版本号（1）将小于新版本（2），下次启动会重新看到新提示。
 *   从未确认过的新用户（0 < 2）同样会看到。
 *   这只改了前端常量与文案，不涉及 config 结构，因此无需 bump `minorVersion`。
 *
 * ▍更新提示文案但无需老用户重看：
 *   只修改 `ScanGuide.vue` 的提示文案，保持本常量不变。已确认过的用户不会重新看到。
 *   只有从未确认过的新用户会看到最新文案。
 */
export const CURRENT_SCAN_TIPS_VERSION: number = 1 as const;

/** `initOeaConfig` 初始化成功前供界面占位。`configInitialized.value === false` 时禁止将这些值写回后端。 */
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

/** 应用内唯一的设置 store，以下导出将其内部状态收窄为各调用方需要的接口。 */
const settings = createConfigStore(DEFAULT_OEA_CONFIG, {
  load: loadOeaConfig,
  save: saveOeaConfig,
  encrypt: cdkEncrypt,
  decrypt: cdkDecrypt,
});

/** 由 `editSettings` 更新的只读设置，保存成功前可以与 `effectiveSettings` 不同。 */
export const draftSettings = settings.draft;
/** `initOeaConfig` 初始化成功或 `saveOeaConfig` 保存成功后更新的只读设置，供自动下载和自动安装读取。 */
export const effectiveSettings = settings.effective;
/** `configInitialized.value === false` 时禁用持久化设置控件，`initOeaConfig` 初始化成功后设为 `true`。 */
export const configInitialized = settings.initialized;
/** 由 `initOeaConfig` 控制，绑定到重新加载按钮的 `loading` 属性。 */
export const configInitializing = settings.initializing;
/** 供设置页展示初始化错误，`initOeaConfig` 开始初始化时重置为 `null`。 */
export const configInitializeError = settings.initializeError;
/** 供设置页和 `App.vue` 展示加密或保存错误，`createConfigStore` 的 `write()` 启动保存时重置为 `null`。 */
export const configSaveError = settings.saveError;
/** 初始化 `settings`。`configInitializeError.value` 非 `null` 时可再次调用以重新加载。 */
export const initOeaConfig = settings.initialize;
/** 更新 `draftSettings` 并安排保存，保存成功后才更新 `effectiveSettings`。 */
export const editSettings = settings.edit;
/** `configSaveError.value` 非 `null` 时重新提交完整 `draftSettings`，供重试按钮调用。 */
export const retrySettingsSave = settings.retry;

/** Nuxt UI 滑块可能发出数组中间值，只接收合法音量。 */
export function setSoundVolume(value: number | number[] | undefined): void {
  if (typeof value === 'number' && Number.isFinite(value)) {
    settings.edit({ soundVolume: Math.min(1, Math.max(0, value)) });
  }
}
