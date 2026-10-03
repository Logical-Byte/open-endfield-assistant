import { effectScope } from 'vue';
import { expect, it, vi } from 'vitest';
import { onAutomationStatusChanged } from '@/features/automation/ipc';
import { initAutomationState } from '@/features/automation/state';
import type { RunEvent, Status } from '@/features/automation/types';
import { onScannedItem } from './ipc';
import { initScannedItems, scannedItems } from './scannedItems';
import type { ScannedItem } from './types/scannedItem';

vi.mock('@/features/automation/ipc', () => ({
  getAutomationStatus: vi.fn(async () => ({ state: 'idle', lastRun: null })),
  onAutomationStatusChanged: vi.fn(),
  startAutomation: vi.fn(),
  stopAutomation: vi.fn(),
}));
vi.mock('./ipc', () => ({ onScannedItem: vi.fn() }));
vi.mock('@/features/gameData/archiveQueries', () => ({ getItemIdsByTitle: vi.fn() }));

it('accepts results for the finished run and discards them once a new scan starts', async () => {
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
    status: 'success',
    foundInCategory: 'document',
    foundInSubCategory: 'paper',
    image: '',
    ocrResult: '研究人员的笔记',
    correctedTitle: '研究人员的笔记',
    correctedMatchItemIds: ['paperNote'],
  };
  try {
    receiveStatus({ state: 'running', taskKind: 'archiveScan', runId: 1 });
    receiveItem({ runId: 1, payload: item });
    receiveStatus({ state: 'stopping', taskKind: 'archiveScan', runId: 1 });
    receiveItem({ runId: 1, payload: item });
    expect(scannedItems.value).toHaveLength(2);
    receiveStatus({
      state: 'idle',
      lastRun: { taskKind: 'archiveScan', runId: 1, outcome: { status: 'completed' } },
    });
    receiveItem({ runId: 1, payload: item });
    expect(scannedItems.value).toHaveLength(3);

    receiveStatus({ state: 'running', taskKind: 'archiveScan', runId: 2 });
    receiveItem({ runId: 1, payload: item });
    expect(scannedItems.value).toHaveLength(0);

    receiveItem({ runId: 2, payload: item });
    expect(scannedItems.value).toHaveLength(1);
  } finally {
    scope.stop();
  }
});
