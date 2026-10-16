import type { ArchiveEntry, ArchiveId } from '@/shared/types/archive';
import type { ScannedItemRecord } from '@/features/archiveScan/types/scannedItem';
import { describe, expect, it } from 'vitest';
import { deriveArchiveCollection } from './collection';

const allItems: ArchiveEntry[] = [
  {
    category: 'paper',
    id: 'paperNote' as ArchiveId,
    acquisitionMethod: 'map',
    title: { 'zh-CN': '研究人员的笔记', 'en-US': '研究人员的笔记' },
  },
  {
    category: 'paper',
    id: 'unrelated' as ArchiveId,
    acquisitionMethod: 'map',
    title: { 'zh-CN': '值班记录', 'en-US': '值班记录' },
  },
  {
    category: 'digital',
    id: 'digitalNote' as ArchiveId,
    acquisitionMethod: 'map',
    title: { 'zh-CN': '研究人员的笔记', 'en-US': '研究人员的笔记' },
  },
];

const successfulScan: ScannedItemRecord = {
  scannedItemId: 1 as ScannedItemRecord['scannedItemId'],
  manualChoice: { kind: 'automatic' },
  gameLocale: 'zh-CN',
  foundInPage: 'document',
  foundInCategory: 'paper',
  image: '',
  ocrResult: '研究人员的笔记',
  matching: { kind: 'exact', archiveId: 'paperNote' as ArchiveId },
};

describe('deriveArchiveCollection', () => {
  it('keeps same-title archives in other subcategories uncollected', () => {
    expect(deriveArchiveCollection(allItems, [successfulScan])).toEqual({
      collectedIds: ['paperNote'],
      notCollectedIds: ['unrelated', 'digitalNote'],
    });
  });

  it('候选为一个或多个时均不计入已收集', () => {
    for (const ids of [['paperNote'], ['paperNote', 'unrelated']]) {
      const scan: ScannedItemRecord = {
        ...successfulScan,
        matching: { kind: 'recommendations', archiveIds: ids as ArchiveId[] },
      };
      expect(deriveArchiveCollection(allItems, [scan])).toEqual({
        collectedIds: [],
        notCollectedIds: ['paperNote', 'unrelated', 'digitalNote'],
      });
    }
  });

  it('人工选择覆盖自动结果，人工清除保持未匹配', () => {
    const manual: ScannedItemRecord = {
      ...successfulScan,
      manualChoice: { kind: 'manual', archiveId: 'unrelated' as ArchiveId },
    };
    expect(deriveArchiveCollection(allItems, [manual])).toEqual({
      collectedIds: ['unrelated'],
      notCollectedIds: ['paperNote', 'digitalNote'],
    });
    const cleared: ScannedItemRecord = {
      ...manual,
      manualChoice: { kind: 'manual', archiveId: null },
    };
    expect(deriveArchiveCollection(allItems, [cleared])).toEqual({
      collectedIds: [],
      notCollectedIds: ['paperNote', 'unrelated', 'digitalNote'],
    });
  });

  it('同一档案的多条证据去重，清除其中一条仍保持已收集', () => {
    const cleared: ScannedItemRecord = {
      ...successfulScan,
      manualChoice: { kind: 'manual', archiveId: null },
    };
    expect(
      deriveArchiveCollection(allItems, [successfulScan, { ...successfulScan }, cleared]),
    ).toEqual({ collectedIds: ['paperNote'], notCollectedIds: ['unrelated', 'digitalNote'] });
  });
});
