import { expect, it, vi } from 'vitest';
import { catalog, catalogError, catalogStatus, initEssenceCatalog } from './catalog';
import { getEssenceCatalog } from './ipc';
import type { EssenceCatalog } from './types';

vi.mock('./ipc', () => ({ getEssenceCatalog: vi.fn() }));

it('shares initialization and exposes a failed load until a retry succeeds', async () => {
  const data: EssenceCatalog = {
    stats: [{ id: 'strength', name: '力量提升', kind: 'attribute' }],
    weapons: [],
  };
  vi.mocked(getEssenceCatalog)
    .mockRejectedValueOnce(new Error('无法加载基质目录'))
    .mockResolvedValueOnce(data);

  expect(catalogStatus.value).toBe('idle');
  expect(getEssenceCatalog).not.toHaveBeenCalled();
  const first = initEssenceCatalog();
  const duplicate = initEssenceCatalog();
  expect(catalogStatus.value).toBe('loading');
  expect(getEssenceCatalog).toHaveBeenCalledTimes(1);
  await Promise.all([first, duplicate]);
  expect(catalogStatus.value).toBe('error');
  expect(catalogError.value).toBe('无法加载基质目录');
  expect(catalog.value).toBeNull();

  const retry = initEssenceCatalog();
  expect(catalogStatus.value).toBe('loading');
  expect(catalogError.value).toBeNull();
  await retry;
  expect(getEssenceCatalog).toHaveBeenCalledTimes(2);
  expect(catalogStatus.value).toBe('ready');
  expect(catalog.value).toEqual(data);
  expect(catalogError.value).toBeNull();
});
