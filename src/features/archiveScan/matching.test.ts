import { describe, expect, it } from 'vitest';
import { deriveArchiveMatching } from './matching';
import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItemRecord } from './types/scannedItem';

const archives: ArchiveEntry[] = [
  {
    id: 'first' as ArchiveId,
    category: 'paper',
    acquisitionMethod: 'map',
    title: { 'zh-CN': '字条', 'en-US': '字条' },
  },
  {
    id: 'second' as ArchiveId,
    category: 'paper',
    acquisitionMethod: 'map',
    title: { 'zh-CN': '字条', 'en-US': '字条' },
  },
  {
    id: 'otherCategory' as ArchiveId,
    category: 'digital',
    acquisitionMethod: 'map',
    title: { 'zh-CN': '字条', 'en-US': '字条' },
  },
];
const firstScan: ScannedItemRecord = {
  scannedItemId: 1 as ScannedItemRecord['scannedItemId'],
  manualChoice: { kind: 'automatic' },
  gameLocale: 'zh-CN',
  foundInPage: 'text',
  foundInCategory: 'paper',
  image: '',
  ocrResult: '字条',
  matching: { kind: 'exact', archiveId: 'first' as ArchiveId },
};

describe('deriveArchiveMatching', () => {
  it('同名档案只关联各自命中的扫描记录', () => {
    const secondScan: ScannedItemRecord = {
      ...firstScan,
      matching: { kind: 'exact', archiveId: 'second' as ArchiveId },
    };
    const matching = deriveArchiveMatching(archives, [firstScan, secondScan]);
    expect(matching.scansByArchiveId.get('first' as ArchiveId)).toEqual([firstScan]);
    expect(matching.scansByArchiveId.get('second' as ArchiveId)).toEqual([secondScan]);
    expect(matching.scansByArchiveId.get('otherCategory' as ArchiveId)).toEqual([]);
    expect(matching.archiveIdByScan.get(firstScan)).toBe('first');
    expect(matching.archiveIdByScan.get(secondScan)).toBe('second');
  });
});
