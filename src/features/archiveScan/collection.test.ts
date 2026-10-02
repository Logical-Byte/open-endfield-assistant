import type { PrtsAllItem } from '@/features/gameData/types/prts';
import type { ScannedItem } from '@/features/archiveScan/types/scannedItem';
import { describe, expect, it } from 'vitest';
import { deriveArchiveCollection } from './collection';

const allItems: Record<string, PrtsAllItem> = {
  paperNote: {
    categoryId: 'paper',
    firstLvId: 'paper-notes',
    id: 'paperNote',
    name: '研究人员的笔记',
    order: 1,
    title: '研究人员的笔记',
    type: 'document',
  },
  unrelated: {
    categoryId: 'paper',
    firstLvId: 'paper-notes',
    id: 'unrelated',
    name: '值班记录',
    order: 2,
    title: '值班记录',
    type: 'document',
  },
  digitalNote: {
    categoryId: 'digital',
    firstLvId: 'digital-notes',
    id: 'digitalNote',
    name: '研究人员的笔记',
    order: 1,
    title: '研究人员的笔记',
    type: 'text',
  },
};

const successfulScan: ScannedItem = {
  status: 'success',
  foundInCategory: 'document',
  foundInSubCategory: 'paper',
  image: '',
  ocrResult: '研究人员的笔记',
  correctedTitle: '研究人员的笔记',
  correctedMatchItemIds: ['paperNote'],
};

describe('deriveArchiveCollection', () => {
  it('keeps same-title archives in other subcategories uncollected', () => {
    expect(deriveArchiveCollection(allItems, [successfulScan])).toEqual({
      collectedIds: ['paperNote'],
      notCollectedIds: ['unrelated', 'digitalNote'],
    });
  });

  it('ignores archive IDs from unsuccessful scans', () => {
    const failedScan: ScannedItem = {
      ...successfulScan,
      status: 'failed',
      correctedMatchItemIds: ['paperNote'],
    };
    const unrecognizedScan: ScannedItem = {
      ...successfulScan,
      status: 'unrecognized',
      correctedMatchItemIds: ['unrelated'],
    };

    expect(deriveArchiveCollection(allItems, [failedScan, unrecognizedScan])).toEqual({
      collectedIds: [],
      notCollectedIds: ['paperNote', 'unrelated', 'digitalNote'],
    });
  });
});
