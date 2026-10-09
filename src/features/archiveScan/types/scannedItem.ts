import type { ScannedItem } from '@/shared/types/archiveScan';
export type { ScannedItem, ScannedItemStatus } from '@/shared/types/archiveScan';

/** 前端会话内的扫描记录身份，与档案 ID 不同，清空后不复用。 */
export type ScannedItemId = number & { readonly __brand: 'ScannedItemId' };

/** 前端编辑状态，原始观察字段来自后端生成契约。 */
export interface ScannedItemRecord extends Omit<ScannedItem, 'correctedMatchItemIds'> {
  correctedMatchItemIds: readonly ScannedItem['correctedMatchItemIds'][number][];
  scannedItemId: ScannedItemId;
  /** 后端自动纠错也会设置 correctedTitle，人工来源单独记录。 */
  manuallyCorrected: boolean;
}
