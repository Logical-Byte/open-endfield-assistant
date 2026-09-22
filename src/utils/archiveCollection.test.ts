import type { PrtsAllItem } from '@/types/prts';
import type { ScanResult } from '@/types/scanResult';
import { describe, expect, it } from 'vitest';
import { deriveArchiveCollection } from './archiveCollection';

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

const successfulScan: ScanResult = {
  status: 'success',
  category: 'document',
  subCategory: 'paper',
  image: '',
  ocrResult: '研究人员的笔记',
  correctedTitle: '研究人员的笔记',
  itemIds: ['paperNote'],
};

describe('deriveArchiveCollection', () => {
  it('collects every archive with the matched title in catalog order', () => {
    expect(deriveArchiveCollection(allItems, [successfulScan])).toEqual({
      collectedIds: ['paperNote', 'digitalNote'],
      notCollectedIds: ['unrelated'],
    });
  });

  it('ignores archive IDs from unsuccessful scans', () => {
    const failedScan: ScanResult = {
      ...successfulScan,
      status: 'failed',
      itemIds: ['paperNote'],
    };
    const unrecognizedScan: ScanResult = {
      ...successfulScan,
      status: 'unrecognized',
      itemIds: ['unrelated'],
    };

    expect(deriveArchiveCollection(allItems, [failedScan, unrecognizedScan])).toEqual({
      collectedIds: [],
      notCollectedIds: ['paperNote', 'unrelated', 'digitalNote'],
    });
  });
});
