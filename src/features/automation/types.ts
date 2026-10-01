/** 用户可以启动的自动化任务种类。 */
export type TaskKind = 'archiveScan';

/** 后端保存的自动化运行状态。 */
export type Status =
  | {
      state: 'idle';
      /** 最近一次运行的结束信息，初始状态为 null，成功启动新任务后移除。 */
      lastRun: { taskKind: TaskKind; outcome: RunOutcome } | null;
    }
  | { state: 'running'; taskKind: TaskKind }
  | { state: 'stopping'; taskKind: TaskKind };

/** 一次自动化运行的终态。 */
export type RunOutcome =
  | { status: 'completed' }
  | { status: 'stopped' }
  | { status: 'failed'; error: string };

/** 自动化运行结束时后端发送的一次性通知。 */
export interface RunFinished {
  taskKind: TaskKind;
  outcome: RunOutcome;
}
