//! 共享实时日志状态（模块级单例 ref，多个组件共享，切页不丢失）。
import { onLog, type LogEntry, type LogLevel } from '@/lib/tauri';
import { computed, ref } from 'vue';

/** 日志等级阈值（数字越小越详细，用于界面过滤）。 */
export const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  TRACE: 0,
  DEBUG: 1,
  INFO: 2,
  WARN: 3,
  ERROR: 4,
};

/** 日志行列表（后端实时追加，全局唯一缓冲） */
export const logLines = ref<LogEntry[]>([]);

/** 界面当前过滤的日志等级（显示该等级及以上） */
export const logLevelFilter = ref<LogLevel>('DEBUG');

/** 按过滤等级筛选后的日志行 */
export const filteredLogLines = computed<LogEntry[]>(() =>
  logLines.value.filter(
    (entry) => LOG_LEVEL_ORDER[entry.level] >= LOG_LEVEL_ORDER[logLevelFilter.value],
  ),
);

/** 日志缓冲上限，防止长时间运行导致内存无限增长 */
const MAX_LINES = 5000;

let initialized = false;

/** 初始化：订阅后端实时日志事件（幂等，全局只需调用一次）。 */
export async function initLogState(): Promise<void> {
  if (initialized) {
    return;
  }
  initialized = true;
  await onLog((entry) => {
    logLines.value.push(entry);
    if (logLines.value.length > MAX_LINES) {
      logLines.value.splice(0, logLines.value.length - MAX_LINES);
    }
  });
}

/** 清空日志缓冲。 */
export function clearLogs(): void {
  logLines.value = [];
}
