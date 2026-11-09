//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { Settings } from '@/features/settings/types/settings';
import { invoke } from '@tauri-apps/api/core';
import { t, type MessageKey } from '@/shared/i18n';
import type { SettingsErrorKind } from '@/shared/types/settings';

export async function loadOeaSettings(): Promise<Settings> {
  try {
    return await invoke('load_oea_settings');
  } catch (error) {
    throw settingsError(error);
  }
}

export async function saveOeaSettings(settings: Settings): Promise<void> {
  try {
    return await invoke('save_oea_settings', { oeaSettings: settings });
  } catch (error) {
    throw settingsError(error);
  }
}

/** 用 DPAPI（当前用户作用域）加密 CDK，返回 Base64 密文。 */
export async function cdkEncrypt(cdk: string): Promise<string> {
  return await invoke<string>('cdk_encrypt', { cdk });
}

/** 用 DPAPI（当前用户作用域）解密 CDK 密文，返回明文。 */
export async function cdkDecrypt(encrypted: string): Promise<string> {
  return await invoke<string>('cdk_decrypt', { encrypted });
}

const errorMessages: Record<SettingsErrorKind, MessageKey> = {
  newerVersion: 'settings.errors.newerVersion',
  unsupportedVersion: 'settings.errors.unsupportedVersion',
  invalidFile: 'settings.errors.invalidFile',
  validation: 'settings.errors.validation',
  read: 'settings.errors.read',
  write: 'settings.errors.write',
  unavailable: 'settings.errors.unavailable',
};

function settingsError(error: unknown): Error {
  if (
    typeof error === 'object' &&
    error !== null &&
    'kind' in error &&
    typeof error.kind === 'string' &&
    Object.prototype.hasOwnProperty.call(errorMessages, error.kind)
  ) {
    const summary = t(errorMessages[error.kind as SettingsErrorKind]);
    const detail = 'message' in error && typeof error.message === 'string' ? error.message : '';
    return new Error(detail ? `${summary}\n${detail}` : summary);
  }
  return error instanceof Error ? error : new Error(String(error));
}
