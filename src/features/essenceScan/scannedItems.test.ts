import { effectScope } from 'vue';
import { expect, it, vi } from 'vitest';
import { onAutomationStatusChanged } from '@/features/automation/ipc';
import { initAutomationState } from '@/features/automation/state';
import type { RunEvent, Status } from '@/features/automation/types';
import { onScannedItem } from './ipc';
import { clearScannedItems, initScannedItems, scanError, scannedItems } from './scannedItems';
import type { ScannedItem } from './types';

vi.mock('@/features/automation/ipc', () => ({
  getAutomationStatus: vi.fn(async () => ({ state: 'idle', lastRun: null })),
  onAutomationStatusChanged: vi.fn(),
  startAutomation: vi.fn(),
  stopAutomation: vi.fn(),
}));
vi.mock('./ipc', () => ({ onScannedItem: vi.fn() }));

it('retains finishing results, protects active scans, and isolates each run and its error', async () => {
  let receiveStatus!: (status: Status) => void;
  let receiveItem!: (event: RunEvent<ScannedItem>) => void;
  vi.mocked(onAutomationStatusChanged).mockImplementation(async (callback) => {
    receiveStatus = callback;
    return () => {};
  });
  vi.mocked(onScannedItem).mockImplementation(async (callback) => {
    receiveItem = callback;
    return () => {};
  });
  await initAutomationState();
  const scope = effectScope();
  await scope.run(initScannedItems);

  const item: ScannedItem = {
    sequence: 1,
    page: 1,
    row: 1,
    column: 1,
    essence: {
      stats: ['strength', 'attack', 'skill'],
      levels: [3, 4, 2],
      rarity: 'five',
      locked: false,
      abandoned: false,
    },
    evaluation: { decision: 'keep', reason: 'weaponMatch', matchedWeaponIds: ['sword'] },
    marking: { status: 'disabled' },
    image: null,
  };
  try {
    receiveStatus({ state: 'running', taskKind: 'essenceScan', runId: 1 });
    receiveItem({ runId: 1, payload: item });
    clearScannedItems();
    expect(scannedItems.value).toHaveLength(1);

    receiveStatus({ state: 'stopping', taskKind: 'essenceScan', runId: 1 });
    receiveItem({ runId: 1, payload: { ...item, sequence: 2 } });
    clearScannedItems();
    expect(scannedItems.value).toHaveLength(2);

    receiveStatus({
      state: 'idle',
      lastRun: { taskKind: 'essenceScan', runId: 1, outcome: { status: 'stopped' } },
    });
    receiveItem({ runId: 1, payload: { ...item, sequence: 3 } });
    expect(scannedItems.value).toHaveLength(3);

    receiveStatus({ state: 'running', taskKind: 'archiveScan', runId: 2 });
    receiveItem({ runId: 2, payload: item });
    expect(scannedItems.value).toHaveLength(3);
    receiveStatus({
      state: 'idle',
      lastRun: { taskKind: 'archiveScan', runId: 2, outcome: { status: 'completed' } },
    });

    receiveStatus({ state: 'running', taskKind: 'essenceScan', runId: 3 });
    receiveItem({ runId: 1, payload: item });
    expect(scannedItems.value).toEqual([]);
    receiveItem({ runId: 3, payload: item });
    receiveStatus({
      state: 'idle',
      lastRun: {
        taskKind: 'essenceScan',
        runId: 3,
        outcome: { status: 'failed', error: '无法读取基质详情' },
      },
    });
    expect(scannedItems.value).toHaveLength(1);
    expect(scanError.value).toBe('无法读取基质详情');

    receiveStatus({ state: 'running', taskKind: 'archiveScan', runId: 4 });
    receiveStatus({
      state: 'idle',
      lastRun: {
        taskKind: 'archiveScan',
        runId: 4,
        outcome: { status: 'failed', error: '档案读取失败' },
      },
    });
    expect(scanError.value).toBe('无法读取基质详情');
    receiveStatus({ state: 'running', taskKind: 'essenceScan', runId: 5 });
    expect(scannedItems.value).toEqual([]);
    expect(scanError.value).toBeNull();
    receiveItem({ runId: 5, payload: item });
    receiveStatus({
      state: 'idle',
      lastRun: { taskKind: 'essenceScan', runId: 5, outcome: { status: 'completed' } },
    });
    clearScannedItems();
    expect(scannedItems.value).toEqual([]);
  } finally {
    scope.stop();
  }
});
