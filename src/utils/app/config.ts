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
