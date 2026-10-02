//! 人工纠错：把用户选择的标题写入扫描结果，标题完全匹配时标记为已收集。

import type { ScannedItem } from '@/features/archiveScan/types/scannedItem';
import { getItemIdsByTitle } from '@/features/gameData/archiveQueries';

/**
 * 应用人工纠错。
 *
 * 标题与当前子分类下的档案完全匹配：标记为已收集（`success`），写入命中的档案 id；
 * 否则视为无法识别（`unrecognized`），清空档案 id，可再次纠正。
 * 当前小分类下同标题多条时全部视为已收集。
 */
export function applyCorrection(scannedItem: ScannedItem, title: string): void {
  scannedItem.correctedTitle = title;
  const correctedMatchItemIds = getItemIdsByTitle(scannedItem.foundInSubCategory, title);
  if (correctedMatchItemIds.length > 0) {
    scannedItem.status = 'success';
    scannedItem.correctedMatchItemIds = correctedMatchItemIds;
  } else {
    scannedItem.status = 'unrecognized';
    scannedItem.correctedMatchItemIds = [];
  }
}
