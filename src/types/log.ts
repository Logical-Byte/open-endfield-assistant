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
