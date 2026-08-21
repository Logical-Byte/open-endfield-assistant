//! 人工纠错：把用户选择的标题写入扫描结果，标题完全匹配时标记为已收集。

import type { ScanResult } from '@/types/scanResult';
import { getItemIdsByTitle } from '@/utils/prts';

/**
 * 应用人工纠错。
 *
 * 标题与当前子分类下的档案完全匹配：标记为已收集（`success`），写入命中的档案 id；
 * 否则视为无法识别（`unrecognized`），清空档案 id，可再次纠正。
 * 同标题多条时全部视为已收集。
 */
export function applyCorrection(scanResult: ScanResult, title: string): void {
  scanResult.correctedTitle = title;
  const itemIds = getItemIdsByTitle(scanResult.subCategory, title);
  if (itemIds.length > 0) {
    scanResult.status = 'success';
    scanResult.itemIds = itemIds;
  } else {
    scanResult.status = 'unrecognized';
    scanResult.itemIds = [];
  }
}
