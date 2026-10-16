//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { Settings } from '@/features/settings/types/settings';
import { invoke } from '@tauri-apps/api/core';

export async function loadOeaSettings(): Promise<Settings> {
  return await invoke('load_oea_settings');
}

export async function saveOeaSettings(settings: Settings): Promise<void> {
  return await invoke('save_oea_settings', { oeaSettings: settings });
}

/** 用 DPAPI（当前用户作用域）加密 CDK，返回 Base64 密文。 */
export async function cdkEncrypt(cdk: string): Promise<string> {
  return await invoke<string>('cdk_encrypt', { cdk });
}

/** 用 DPAPI（当前用户作用域）解密 CDK 密文，返回明文。 */
export async function cdkDecrypt(encrypted: string): Promise<string> {
  return await invoke<string>('cdk_decrypt', { encrypted });
}
