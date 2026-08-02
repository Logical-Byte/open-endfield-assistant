//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

/** 后端返回的应用状态（与 Rust 侧 `AppStatus` 对齐）。 */
export interface AppStatus {
  /** 主任务是否正在运行 */
  running: boolean
}

/** 启动档案库主任务（后端在后台线程执行，立即返回当前状态）。 */
export function startScan(): Promise<AppStatus> {
  return invoke('start_scan')
}

/** 请求停止主任务（优雅停止）。 */
export function stopScan(): Promise<AppStatus> {
  return invoke('stop_scan')
}

/** 单次扫描当前档案详情（仅截屏识别）。 */
export function scanSingle(): Promise<AppStatus> {
  return invoke('scan_single')
}

/** 查询当前应用状态。 */
export function getStatus(): Promise<AppStatus> {
  return invoke('get_status')
}

/** 退出程序。 */
export function quitApp(): Promise<void> {
  return invoke('quit')
}

/**
 * 监听应用状态变更事件（启动 / 结束均触发，payload 为最新 AppStatus）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export function onAppStatus(cb: (status: AppStatus) => void): Promise<() => void> {
  return listen<AppStatus>('app-status', (event) => cb(event.payload))
}

/**
 * 监听后端实时日志（每行一个字符串）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export function onLog(cb: (line: string) => void): Promise<() => void> {
  return listen<string>('log', (event) => cb(event.payload))
}
