//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { DownloadOutcome } from '@/shared/types/update';
import type { DownloadProgress, UpdateAvailability, UpdateStatus } from './types/update';
import { Channel, invoke } from '@tauri-apps/api/core';

export async function getUpdateStatus(): Promise<UpdateStatus> {
  return await invoke<UpdateStatus>('get_update_status');
}

export async function requestUpdateCheck(): Promise<UpdateAvailability> {
  return await invoke<UpdateAvailability>('check_update');
}

export function createDownloadProgressChannel(
  cb: (progress: DownloadProgress) => void,
): Channel<DownloadProgress> {
  return new Channel<DownloadProgress>(cb);
}

export async function downloadUpdate(
  onProgress: Channel<DownloadProgress>,
): Promise<DownloadOutcome> {
  return await invoke<DownloadOutcome>('download_update', { onProgress });
}

export async function installUpdate(restart: boolean): Promise<void> {
  await invoke('install_update', { restart });
}

export async function developerInstallUpdate(): Promise<void> {
  return await invoke<void>('developer_install_update');
}

export async function selectDeveloperUpdate(): Promise<boolean> {
  return await invoke<boolean>('select_developer_update');
}
export async function discardDeveloperUpdate(): Promise<void> {
  await invoke('discard_developer_update');
}
