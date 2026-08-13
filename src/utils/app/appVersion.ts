import { getVersion } from '@tauri-apps/api/app';
import { isTauri } from '@tauri-apps/api/core';
import { ref } from 'vue';

/** 应用版本号（如 "0.1.0"；非 Tauri 环境为 null）。 */
export const appVersion = ref<string | null>(null);

export async function initAppVersion() {
  // 仅 Tauri 环境存在版本信息；浏览器调试环境不显示版本号。
  if (isTauri()) {
    appVersion.value = await getVersion();
  }
}
