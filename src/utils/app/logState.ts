import type { LogEntry } from '@/types/log';
import { onLog } from '@/utils/tauri';
import { ref } from 'vue';

/** 日志缓冲上限，防止长时间运行导致内存无限增长 */
export const MAX_LINES = 4096;

/** 日志行列表（后端实时追加，全局唯一缓冲） */
export const logLines = ref<LogEntry[]>([]);

let initialized = false;

export function initLogState() {
  if (!initialized) {
    initialized = true;

    onLog((logEntry) => {
      logLines.value.push(logEntry);
      if (logLines.value.length > MAX_LINES) {
        logLines.value.splice(0, logLines.value.length - MAX_LINES);
      }
    });
  }
}
