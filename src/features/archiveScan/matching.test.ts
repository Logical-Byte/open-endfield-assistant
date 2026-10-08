import { describe, expect, it } from 'vitest';
import { deriveArchiveMatching } from './matching';
import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ArchiveId, ScannedItem } from './types/scannedItem';

const archives: Record<string, PrtsAllItem> = {
  first: {
    id: 'first',
    categoryId: 'paper',
    firstLvId: 'notes',
    name: '字条',
    title: '字条',
    order: 1,
    type: 'text',
  },
  second: {
    id: 'second',
    categoryId: 'paper',
    firstLvId: 'notes',
    name: '字条',
    title: '字条',
    order: 2,
    type: 'text',
  },
  otherCategory: {
    id: 'otherCategory',
    categoryId: 'digital',
    firstLvId: 'digital',
    name: '字条',
    title: '字条',
    order: 1,
    type: 'text',
  },
};
const firstScan: ScannedItem = {
  status: 'success',
  foundInCategory: 'text',
  foundInSubCategory: 'paper',
  image: '',
  ocrResult: '字条',
  correctedTitle: '字条',
  correctedMatchItemIds: ['first'],
};

describe('deriveArchiveMatching', () => {
  it('保留多条扫描与同分类同名档案的全部双向关联', () => {
    const secondScan: ScannedItem = { ...firstScan, correctedMatchItemIds: ['second'] };
    const matching = deriveArchiveMatching(archives, [firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('first' as ArchiveId)).toEqual([firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('second' as ArchiveId)).toEqual([firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('otherCategory' as ArchiveId)).toEqual([]);
    expect(matching.archiveIdsByScan.get(firstScan)).toEqual(['first', 'second']);
    expect(matching.archiveIdsByScan.get(secondScan)).toEqual(['first', 'second']);
  });
});
