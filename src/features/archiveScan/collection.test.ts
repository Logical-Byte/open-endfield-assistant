import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItem } from '@/features/archiveScan/types/scannedItem';
import { describe, expect, it } from 'vitest';
import { deriveArchiveCollection } from './collection';

const allItems: ArchiveEntry[] = [
  {
    category: 'paper',
    id: 'paperNote' as ArchiveId,
    acquisitionMethod: 'map',
    title: '研究人员的笔记',
  },
  {
    category: 'paper',
    id: 'unrelated' as ArchiveId,
    acquisitionMethod: 'map',
    title: '值班记录',
  },
  {
    category: 'digital',
    id: 'digitalNote' as ArchiveId,
    acquisitionMethod: 'map',
    title: '研究人员的笔记',
  },
];

const successfulScan: ScannedItem = {
  status: 'success',
  foundInPage: 'document',
  foundInCategory: 'paper',
  image: '',
  ocrResult: '研究人员的笔记',
  correctedTitle: '研究人员的笔记',
  correctedMatchItemIds: ['paperNote' as ArchiveId],
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
      correctedMatchItemIds: ['paperNote' as ArchiveId],
    };
    const unrecognizedScan: ScannedItem = {
      ...successfulScan,
      status: 'unrecognized',
      correctedMatchItemIds: ['unrelated' as ArchiveId],
    };

    expect(deriveArchiveCollection(allItems, [failedScan, unrecognizedScan])).toEqual({
      collectedIds: [],
      notCollectedIds: ['paperNote', 'unrelated', 'digitalNote'],
    });
  });
});
