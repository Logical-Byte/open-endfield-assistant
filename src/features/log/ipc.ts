//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { LogEntry } from '@/features/log/types/log';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** 在系统文件管理器中打开日志目录（后端通过 opener 插件执行）。 */
export async function openLogDir(): Promise<void> {
  await invoke('open_log_dir');
}

/**
 * 监听后端实时日志（每条含等级与文本）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export async function onLog(cb: (entry: LogEntry) => void): Promise<() => void> {
  return await listen<LogEntry>('log', (event) => cb(event.payload));
}

/** 写一条 TRACE 级日志到后端日志系统，并同步打印到浏览器控制台。 */
export async function logTrace(message: string): Promise<void> {
  console.debug(message);
  await invoke('log_trace', { message });
}

/** 写一条 DEBUG 级日志到后端日志系统，并同步打印到浏览器控制台。 */
export async function logDebug(message: string): Promise<void> {
  console.debug(message);
  await invoke('log_debug', { message });
}

/** 写一条 INFO 级日志到后端日志系统，并同步打印到浏览器控制台。 */
export async function logInfo(message: string): Promise<void> {
  console.info(message);
  await invoke('log_info', { message });
}

/** 写一条 WARN 级日志到后端日志系统，并同步打印到浏览器控制台。 */
export async function logWarn(message: string): Promise<void> {
  console.warn(message);
  await invoke('log_warn', { message });
}

/** 写一条 ERROR 级日志到后端日志系统，并同步打印到浏览器控制台。 */
export async function logError(message: string): Promise<void> {
  console.error(message);
  await invoke('log_error', { message });
}
