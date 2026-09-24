import { type DeepReadonly } from 'vue';

import type { SettingsDraft, SettingsSnapshot, SettingsStatus } from './model';
import { createTauriSettingsPersistence } from './persistence';
import { createSettingsModule } from './settings';

export {
  proxyModeItems,
  updateSourceItems,
  UpdateProxyMode,
  UpdateSource,
  type SettingsDraft,
  type SettingsSnapshot,
  type SettingsStatus,
} from './model';

const settings = createSettingsModule(createTauriSettingsPersistence());

/** 用户当前编辑意图；字段赋值会同步进入校验和 single writer 调度。 */
export const settingsDraft: SettingsDraft = settings.settingsDraft;

/** 最近一次成功保存并已生效的只读设置投影。 */
export const effectiveSettings: DeepReadonly<SettingsSnapshot> = settings.effectiveSettings;

/** Settings 单例当前的初始化、保存或错误状态。 */
export const settingsStatus: DeepReadonly<SettingsStatus> = settings.settingsStatus;

/** 从持久化配置初始化生产 settings 单例；重复调用会复用同一次初始化。 */
export const initializeSettings: () => Promise<void> = settings.initializeSettings;

/** 重新提交当前失败的完整 draft。 */
export const retrySettingsSave: () => void = settings.retrySettingsSave;

/** 放弃完整 draft，并恢复到最近一次成功保存的 effective 设置。 */
export const discardSettingsDraft: () => void = settings.discardSettingsDraft;

export { initUiScale, uiScale } from './uiScale';
