//! 共享实时日志状态（模块级单例 ref，多个组件共享，切页不丢失）。
import { onLog } from '@/lib/tauri';
import { ref } from 'vue';

/** 日志行列表（后端实时追加，全局唯一缓冲） */
export const logLines = ref<string[]>([]);

/** 日志缓冲上限，防止长时间运行导致内存无限增长 */
const MAX_LINES = 5000;

let initialized = false;

/** 初始化：订阅后端实时日志事件（幂等，全局只需调用一次）。 */
export async function initLogState(): Promise<void> {
  if (initialized) {
    return;
  }
  initialized = true;
  await onLog((line) => {
    logLines.value.push(line);
    if (logLines.value.length > MAX_LINES) {
      logLines.value.splice(0, logLines.value.length - MAX_LINES);
    }
  });
}

/** 清空日志缓冲。 */
export function clearLogs(): void {
  logLines.value = [];
}
