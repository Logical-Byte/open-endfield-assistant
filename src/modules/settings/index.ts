import { type DeepReadonly } from 'vue';

import type { SettingsDraft, SettingsSnapshot, SettingsStatus } from './model';
import { createTauriSettingsPersistence } from './persistence';
import { createSettingsModule } from './settings';

const settings = createSettingsModule(createTauriSettingsPersistence());

export const settingsDraft: SettingsDraft = settings.settingsDraft;
export const effectiveSettings: DeepReadonly<SettingsSnapshot> = settings.effectiveSettings;
export const settingsStatus: DeepReadonly<SettingsStatus> = settings.settingsStatus;

export const initializeSettings: () => Promise<void> = settings.initializeSettings;
export const retrySettingsSave: () => void = settings.retrySettingsSave;
export const discardSettingsDraft: () => void = settings.discardSettingsDraft;
