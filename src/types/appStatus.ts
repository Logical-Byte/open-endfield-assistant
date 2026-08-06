/** 后端返回的应用状态（与 Rust 侧 `AppStatus` 对齐）。 */

export interface AppStatus {
  /** 主任务是否正在运行 */
  running: boolean;
}
