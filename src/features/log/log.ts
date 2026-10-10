import { t, type MessageKey } from '@/shared/i18n';
import type { LogEntry, LogLevel } from '@/features/log/types/log';
import { logLines } from '@/features/log/logState';
import { computed, ref } from 'vue';

/** 日志等级阈值（数字越小越详细，用于界面过滤）。 */
const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  TRACE: 0,
  DEBUG: 1,
  INFO: 2,
  WARN: 3,
  ERROR: 4,
};

/** 展示标签与过滤使用的日志等级事实分离。 */
export const levelLabels: Record<LogLevel, MessageKey> = {
  TRACE: 'log.level.trace',
  DEBUG: 'log.level.debug',
  INFO: 'log.level.info',
  WARN: 'log.level.warn',
  ERROR: 'log.level.error',
};
export const levelOptions = computed((): { value: LogLevel; label: string }[] =>
  (Object.keys(levelLabels) as LogLevel[]).map(
    (value: LogLevel): { value: LogLevel; label: string } => ({
      value,
      label: t(levelLabels[value]),
    }),
  ),
);

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
