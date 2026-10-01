//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type {
  DownloadProgress,
  UpdateAvailability,
  UpdateInfo,
  UpdateInstallStageEvent,
  UpdateStatus,
} from './types/update';
import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';

/** Rust 启动恢复的最小返回值：完成一次资源事务，或没有已完成的事务。 */
export type StartupUpdateResult = 'completed' | null;

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

export async function downloadUpdate(onProgress: Channel<DownloadProgress>): Promise<UpdateInfo> {
  return await invoke<UpdateInfo>('download_update', { onProgress });
}

export async function requestDownloadCancellation(): Promise<void> {
  await invoke('cancel_download');
}

export async function takeStartupUpdateResult(): Promise<StartupUpdateResult> {
  return await invoke<StartupUpdateResult>('consume_startup_update_result');
}

export async function onUpdateInstallStage(
  cb: (event: Event<UpdateInstallStageEvent>) => void,
): Promise<() => void> {
  return await listen<UpdateInstallStageEvent>('update-install-stage', cb);
}

export async function installUpdate(): Promise<void> {
  await invoke('install_update');
}

export async function developerInstallUpdate(): Promise<boolean> {
  return await invoke<boolean>('developer_install_update');
}
