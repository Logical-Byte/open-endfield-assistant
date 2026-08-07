import type { LogEntry, LogLevel } from '@/types/log';
import { logLines } from '@/utils/app/logState';
import { computed, ref } from 'vue';

/** 日志等级阈值（数字越小越详细，用于界面过滤）。 */
const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  TRACE: 0,
  DEBUG: 1,
  INFO: 2,
  WARN: 3,
  ERROR: 4,
};

/** 日志等级过滤选项（显示该等级及以上）。 */
export const levelOptions: { label: string; value: LogLevel }[] = [
  { label: 'TRACE', value: 'TRACE' },
  { label: 'DEBUG', value: 'DEBUG' },
  { label: 'INFO', value: 'INFO' },
  { label: 'WARN', value: 'WARN' },
  { label: 'ERROR', value: 'ERROR' },
] as const;

/** 界面当前过滤的日志等级（显示该等级及以上） */
export const logLevelFilter = ref<LogLevel>('INFO');

/** 按过滤等级筛选后的日志行 */
export const filteredLogLines = computed<LogEntry[]>(() =>
  logLines.value.filter(
    (entry) => LOG_LEVEL_ORDER[entry.level] >= LOG_LEVEL_ORDER[logLevelFilter.value],
  ),
);

export function clearLogs() {
  logLines.value = [];
}
