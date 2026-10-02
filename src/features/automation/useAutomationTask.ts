import { computed, shallowReadonly, shallowRef, type ComputedRef, type Ref } from 'vue';
import { whenever } from '@vueuse/core';
import { startAutomation, stopAutomation } from './ipc';
import { automationStatus } from './state';
import type { RunOutcome, StartRequest, TaskKind } from './types';

/** 从当前绑定的 taskKind 视角观察的后端任务阶段。 */
export type TaskPhase =
  /** 全局没有自动化任务运行。可以请求开始绑定的 taskKind。 */
  | 'idle'
  /** 当前运行的 taskKind 与绑定的 taskKind 相同，在正常运行。可以请求它停止。 */
  | 'running'
  /** 当前运行的 taskKind 与绑定的 taskKind 相同，在停止中。 */
  | 'stopping'
  /** 其他种类的任务正在运行或停止。绑定的 taskKind 不能启动，也不能停止其他任务。 */
  | 'blocked';

/** 已绑定 taskKind 后，调用方需要提供的启动参数。 */
export type StartOptions<K extends TaskKind> = Omit<
  Extract<StartRequest, { taskKind: K }>,
  'taskKind'
>;

/** submitted 表示 IPC 正常返回。skipped 表示当前的 TaskPhase 不允许该操作。 */
export type CommandResult = 'submitted' | 'skipped';

/**
 * 固定任务种类的控制接口。多个调用方观察同一后端状态而无需直接接触 automationStatus 细节。
 */
export interface AutomationTask<K extends TaskKind> {
  /** 只读后端状态投影。*/
  readonly phase: ComputedRef<TaskPhase>;
  /** 当前绑定的任务仍未结束，包含运行和停止收尾阶段。 */
  readonly isActive: ComputedRef<boolean>;
  /**
   * 可观察到的最近一次的绑定的 taskKind 的运行结果，尚未记录时为 null。
   * 每次 composable 调用独立保留自己的记录。
   */
  readonly outcome: Readonly<Ref<Readonly<RunOutcome> | null>>;
  /** 全局空闲时允许启动。 */
  readonly canStart: ComputedRef<boolean>;
  /** 仅绑定的 taskKind running 时允许停止。stopping 时为 false，不允许再次请求。 */
  readonly canStop: ComputedRef<boolean>;
  /** 请求开始绑定的 taskKind 任务。 */
  tryStart(options: StartOptions<K>): Promise<CommandResult>;
  /** 请求结束绑定的 taskKind 任务。 */
  tryStop(): Promise<CommandResult>;
}

/**
 * 绑定 taskKind，过滤 automationStatus 中只与当前 taskKind 有关的事件。
 * 提供了绑定 taskKind 调用者关注的相关事件、能否请求启动/停止，以及尊重 automationStatus 单并发控制的请求启动/停止函数。*/
export function useAutomationTask<K extends TaskKind>(taskKind: K): AutomationTask<K> {
  const phase = computed<TaskPhase>((): TaskPhase => {
    const status = automationStatus.value;
    if (status.state === 'idle') return 'idle';
    return status.taskKind === taskKind ? status.state : 'blocked';
  });
  const canStart = computed((): boolean => phase.value === 'idle');
  const isActive = computed((): boolean => phase.value === 'running' || phase.value === 'stopping');
  const canStop = computed((): boolean => phase.value === 'running');
  const outcome = shallowRef<Readonly<RunOutcome> | null>(null);
  whenever(
    (): Readonly<RunOutcome> | null => {
      const status = automationStatus.value;
      return status.state === 'idle' && status.lastRun?.taskKind === taskKind
        ? status.lastRun.outcome
        : null;
    },
    (nextOutcome) => {
      outcome.value = nextOutcome;
    },
    // 立即读取已有结果，同步记录后续结束状态，避免快速启动下一任务时漏掉结果。
    { immediate: true, flush: 'sync' },
  );

  async function tryStart(options: StartOptions<K>): Promise<CommandResult> {
    if (!canStart.value) return 'skipped';
    // 泛型的 Extract/Omit 重组无法由 TS 证明，参数关联已由公开接口约束。
    const request = { ...options, taskKind } as StartRequest;
    await startAutomation(request);
    return 'submitted';
  }

  async function tryStop(): Promise<CommandResult> {
    if (!canStop.value) return 'skipped';
    await stopAutomation();
    return 'submitted';
  }

  return {
    phase,
    isActive,
    outcome: shallowReadonly(outcome),
    canStart,
    canStop,
    tryStart,
    tryStop,
  };
}
