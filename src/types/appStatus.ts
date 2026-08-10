/** 后端返回的应用状态（与 Rust 侧 `AppStatus` 对齐）。 */

export interface AppStatus {
  /** 扫描档案库任务是否正在运行 */
  running: boolean;
  /** 最近一次扫描档案库任务的失败原因（成功 / 被停止时为 null，每次启动任务时清空） */
  scanError: string | null;
}
