//! Tauri 后端接口封装：类型安全地调用 Rust 命令、监听后端事件。

import type * as Automation from '@/features/automation/types';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** 启动指定种类的自动化任务（后端在后台线程执行，立即返回当前状态）。 */
export async function startAutomation(
  request: Automation.StartRequest,
): Promise<Automation.Status> {
  return await invoke('start_automation', { request });
}

/** 请求停止当前自动化任务（优雅停止）。 */
export async function stopAutomation(): Promise<Automation.Status> {
  return await invoke('stop_automation');
}

/** 查询当前自动化状态。 */
export async function getAutomationStatus(): Promise<Automation.Status> {
  return await invoke('get_automation_status');
}

/**
 * 监听自动化状态变更事件（启动 / 结束均触发）。
 * 返回取消监听函数，组件卸载时应调用。
 */
export async function onAutomationStatusChanged(
  cb: (status: Automation.Status) => void,
): Promise<() => void> {
  return await listen<Automation.Status>('automation-status-changed', (event) => cb(event.payload));
}
