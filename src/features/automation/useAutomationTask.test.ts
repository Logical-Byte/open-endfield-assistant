import { effectScope } from 'vue';
import { expect, it, vi } from 'vitest';
import { onAutomationStatusChanged, startAutomation, stopAutomation } from './ipc';
import { initAutomationState } from './state';
import type { Status } from './types';
import { useAutomationTask } from './useAutomationTask';

vi.mock('./ipc', () => ({
  getAutomationStatus: vi.fn(async () => ({ state: 'idle', lastRun: null })),
  onAutomationStatusChanged: vi.fn(),
  startAutomation: vi.fn(),
  stopAutomation: vi.fn(),
}));

it('blocks cross-task commands while retaining the previous essence run', async () => {
  let receiveStatus!: (status: Status) => void;
  vi.mocked(onAutomationStatusChanged).mockImplementation(async (callback) => {
    receiveStatus = callback;
    return () => {};
  });
  await initAutomationState();
  const scope = effectScope();
  const task = scope.run(() => useAutomationTask('essenceScan'))!;
  try {
    expect(await task.tryStart({ workerType: 'simulation' })).toBe('submitted');
    expect(startAutomation).toHaveBeenCalledWith({
      taskKind: 'essenceScan',
      workerType: 'simulation',
    });
    receiveStatus({ state: 'running', taskKind: 'essenceScan', runId: 10 });
    expect(task.runId.value).toBe(10);
    expect(await task.tryStop()).toBe('submitted');
    receiveStatus({ state: 'stopping', taskKind: 'essenceScan', runId: 10 });
    expect(task.isActive.value).toBe(true);
    expect(await task.tryStop()).toBe('skipped');
    receiveStatus({
      state: 'idle',
      lastRun: { taskKind: 'essenceScan', runId: 10, outcome: { status: 'stopped' } },
    });

    for (const state of ['running', 'stopping'] as const) {
      receiveStatus({ state, taskKind: 'archiveScan', runId: 11 });
      expect(task.phase.value).toBe('blocked');
      expect(task.isActive.value).toBe(false);
      expect(task.canStart.value).toBe(false);
      expect(task.canStop.value).toBe(false);
      expect(await task.tryStart({ workerType: 'simulation' })).toBe('skipped');
      expect(await task.tryStop()).toBe('skipped');
      expect(task.runId.value).toBe(10);
      expect(task.outcome.value).toEqual({ status: 'stopped' });
    }
    expect(startAutomation).toHaveBeenCalledTimes(1);
    expect(stopAutomation).toHaveBeenCalledTimes(1);
  } finally {
    scope.stop();
  }
});
