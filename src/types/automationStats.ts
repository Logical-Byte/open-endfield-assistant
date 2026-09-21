/** capability 方法在一次捕获区间内的调用次数。 */
export interface CapabilityCallCounts {
  screenshot: number;
  click: number;
  pressKey: number;
  moveMouseToSafePosition: number;
  findTemplate: number;
  recognizeText: number;
  sleep: number;
}

/** 一次 capability 计数捕获的统计摘要。 */
export interface CaptureSummary {
  /** 捕获区间的单调时钟墙上时间，单位为微秒。 */
  elapsedMicros: number;
  calls: CapabilityCallCounts;
}

export type AutomationTask = 'archiveScan';
export type AutomationOutcome = 'completed' | 'stopped' | 'failed';

/** 后端在一次自动化运行到达终态时推送的事件。 */
export interface AutomationRunFinished {
  task: AutomationTask;
  outcome: AutomationOutcome;
  /** 连接 Session 前失败时没有建立捕获区间。 */
  capture: CaptureSummary | null;
}
