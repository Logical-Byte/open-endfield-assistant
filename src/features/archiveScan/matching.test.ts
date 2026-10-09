import { describe, expect, it } from 'vitest';
import { deriveArchiveMatching } from './matching';
import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItem } from './types/scannedItem';

const archives: ArchiveEntry[] = [
  {
    id: 'first' as ArchiveId,
    category: 'paper',
    acquisitionMethod: 'map',
    title: '字条',
  },
  {
    id: 'second' as ArchiveId,
    category: 'paper',
    acquisitionMethod: 'map',
    title: '字条',
  },
  {
    id: 'otherCategory' as ArchiveId,
    category: 'digital',
    acquisitionMethod: 'map',
    title: '字条',
  },
];
const firstScan: ScannedItem = {
  status: 'success',
  foundInPage: 'text',
  foundInCategory: 'paper',
  image: '',
  ocrResult: '字条',
  correctedTitle: '字条',
  correctedMatchItemIds: ['first' as ArchiveId],
};

describe('deriveArchiveMatching', () => {
  it('保留多条扫描与同分类同名档案的全部双向关联', () => {
    const secondScan: ScannedItem = {
      ...firstScan,
      correctedMatchItemIds: ['second' as ArchiveId],
    };
    const matching = deriveArchiveMatching(archives, [firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('first' as ArchiveId)).toEqual([firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('second' as ArchiveId)).toEqual([firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('otherCategory' as ArchiveId)).toEqual([]);
    expect(matching.archiveIdsByScan.get(firstScan)).toEqual(['first', 'second']);
    expect(matching.archiveIdsByScan.get(secondScan)).toEqual(['first', 'second']);
  });
});
