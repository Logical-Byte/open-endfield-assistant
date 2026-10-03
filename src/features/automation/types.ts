import type { ArchiveScanWorkerType } from '@/features/archiveScan/workerType';

export type StartRequest = {
  taskKind: 'archiveScan';
  workerType: ArchiveScanWorkerType;
};

/** 用户可以启动的自动化任务种类。 */
export type TaskKind = StartRequest['taskKind'];

/** 后端保存的自动化运行状态。 */
export type Status =
  | {
      state: 'idle';
      /** 最近一次运行的结束信息，初始状态为 null，成功启动新任务后移除。 */
      lastRun: { runId: number; taskKind: TaskKind; outcome: RunOutcome } | null;
    }
  | { state: 'running'; runId: number; taskKind: TaskKind }
  | { state: 'stopping'; runId: number; taskKind: TaskKind };

/** 领域事件所属的运行，用于排除上一轮延迟送达的结果。 */
export interface RunEvent<T> {
  runId: number;
  payload: T;
}

/** 一次自动化运行的终态。 */
export type RunOutcome =
  | { status: 'completed' }
  | { status: 'stopped' }
  | { status: 'failed'; error: string };
