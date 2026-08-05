//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** 后端返回的应用状态（与 Rust 侧 `AppStatus` 对齐）。 */
export interface AppStatus {
  /** 主任务是否正在运行 */
  running: boolean;
}

/** 单份档案的扫描结果（与 Rust 侧 `ScanResult` 对齐）。 */
export interface ScanResult {
  /** 识别状态：success（OCR 结果非空）| failed（OCR 结果为空） */
  status: 'success' | 'failed';
  /** 全局序号（从 1 开始，跨分类连续递增） */
  index: number;
  /** 档案库分类：音像存档 / 见闻辑录 / 中枢档案 */
  category: string;
  /** 档案详情页面截图（base64 PNG data URL） */
  image: string;
  /** OCR 识别结果（前端可编辑） */
  ocr_result: string;
}

/** 后端日志等级（与 Rust 侧 `tracing::Level` 对齐）。 */
export type LogLevel = 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

/** 后端推送的单条日志（与 Rust 侧 `LogEntry` 对齐）。 */
export interface LogEntry {
  /** 时间（本地时间，`MM-dd HH:MM:SS`） */
  time: string;
  /** 日志等级：TRACE / DEBUG / INFO / WARN / ERROR */
  level: LogLevel;
  /** 格式化后的日志文本 */
  message: string;
}

/** 启动档案库主任务（后端在后台线程执行，立即返回当前状态）。 */
export async function startScan(): Promise<AppStatus> {
  return await invoke('start_scan');
}

/** 请求停止主任务（优雅停止）。 */
export async function stopScan(): Promise<AppStatus> {
  return await invoke('stop_scan');
}

/** 单次扫描当前档案详情（仅截屏识别）。 */
export async function scanSingle(): Promise<AppStatus> {
  return await invoke('scan_single');
}

/** 查询当前应用状态。 */
export async function getStatus(): Promise<AppStatus> {
  return await invoke('get_status');
}

/** 退出程序。 */
export async function quitApp(): Promise<void> {
  return await invoke('quit');
}

/** 在系统文件管理器中打开日志目录（后端通过 opener 插件执行）。 */
export async function openLogDir(): Promise<void> {
  await invoke('open_log_dir');
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
