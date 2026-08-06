//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type { AppStatus } from '@/types/appStatus';
import type { LogEntry } from '@/types/log';
import type { PrtsData } from '@/types/prts';
import type { ScanResult } from '@/types/scanResult';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** 启动扫描档案库任务（后端在后台线程执行，立即返回当前状态）。 */
export async function startScan(): Promise<AppStatus> {
  return await invoke('start_scan');
}

/** 请求停止扫描档案库任务（优雅停止）。 */
export async function stopScan(): Promise<AppStatus> {
  return await invoke('stop_scan');
}

/** 查询当前应用状态。 */
export async function getStatus(): Promise<AppStatus> {
  return await invoke('get_status');
}

/** 获取 prts.json 完整数据（分类中文名映射 / 自动补全候选）。 */
export async function getPrtsData(): Promise<PrtsData> {
  return await invoke('get_prts_data');
}

/** 退出程序。 */
export async function quitApp(): Promise<void> {
  return await invoke('quit');
}

/** 在系统文件管理器中打开日志目录（后端通过 opener 插件执行）。 */
export async function openLogDir(): Promise<void> {
  await invoke('open_log_dir');
}

/** 查询"关闭窗口时最小化到托盘"。 */
export async function getMinimizeToTray(): Promise<boolean> {
  return await invoke('get_minimize_to_tray');
}

/** 设置"关闭窗口时最小化到托盘"（返回设置后的实际值）。 */
export async function setMinimizeToTray(enabled: boolean): Promise<boolean> {
  return await invoke('set_minimize_to_tray', { enabled });
}

/**
 * 监听应用状态变更事件（启动 / 结束均触发，payload 为最新 AppStatus）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export async function onAppStatus(cb: (status: AppStatus) => void): Promise<() => void> {
  return await listen<AppStatus>('app-status', (event) => cb(event.payload));
}

/**
 * 监听后端实时日志（每条含等级与文本）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export async function onLog(cb: (entry: LogEntry) => void): Promise<() => void> {
  return await listen<LogEntry>('log', (event) => cb(event.payload));
}

/**
 * 监听后端扫描结果事件（扫描进度中每识别一份档案触发一次）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export async function onScanResult(cb: (result: ScanResult) => void): Promise<() => void> {
  return await listen<ScanResult>('scan-result', (event) => cb(event.payload));
}
