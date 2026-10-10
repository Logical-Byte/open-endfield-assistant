import { t } from '@/shared/i18n';
import type { UpdateProxyMode, UpdateSource } from '@/features/settings/types/settings';
import { cdkDecrypt, cdkEncrypt, loadOeaSettings, saveOeaSettings } from '@/features/settings/ipc';
import { computed } from 'vue';
import { createSettingsStore } from './settingsStore';

/** 更新源选项 */
export const updateSourceItems = computed<{ label: string; value: UpdateSource }[]>(() => [
  { label: t('settings.update.source.mirror'), value: 'mirrorchyan' },
  { label: 'OEM', value: 'oem' },
  { label: 'GitHub', value: 'github' },
]);

/** 更新代理模式选项 */
export const proxyModeItems = computed<{ label: string; value: UpdateProxyMode }[]>(() => [
  { label: t('settings.update.proxy.none'), value: 'none' },
  { label: t('settings.update.proxy.system'), value: 'system' },
  { label: t('settings.update.proxy.custom'), value: 'custom' },
]);

/** 应用内唯一的设置 store。完整初始值只由 Rust 后端返回。 */
const settings = createSettingsStore({
  load: loadOeaSettings,
  save: saveOeaSettings,
  encrypt: cdkEncrypt,
  decrypt: cdkDecrypt,
});

/** Settings 初始化生命周期，以及 ready 后的 draft/effective 设置。 */
export const settingsState = settings.state;
/** 供应用层统一展示保存失败通知。 */
export const settingsSaveError = computed(() =>
  settingsState.value.status === 'ready' ? settingsState.value.saveError : null,
);
/** 从 Rust 后端初始化完整设置，初始化失败后可再次调用。 */
export const initOeaSettings = settings.initialize;
/** 标记纯浏览器模式不支持后端 Settings。 */
export const markSettingsUnsupported = settings.markUnsupported;
/** 更新 ready 状态内的 draft，保存成功后才更新 effective。 */
export const editSettings = settings.edit;
/** 保存失败后重新提交完整 draft。 */
export const retrySettingsSave = settings.retry;
