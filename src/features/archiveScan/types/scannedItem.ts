import type { DeepReadonly } from 'vue';
import type { ScannedItem } from '@/shared/types/archiveScan';
import type { ArchiveId } from '@/shared/types/archive';
export type { ScannedItem } from '@/shared/types/archiveScan';

/** 前端会话内的扫描记录身份，与档案 ID 不同，清空后不复用。 */
export type ScannedItemId = number & { readonly __brand: 'ScannedItemId' };

/** 没有人工决定与明确的人工未选择必须分开。 */
export type ManualChoice =
  | { readonly kind: 'automatic' }
  | { readonly kind: 'manual'; readonly archiveId: ArchiveId | null };

/** 原始观察和自动结果保留，人工决定独立覆盖自动匹配。 */
export interface ScannedItemRecord extends Omit<ScannedItem, 'matching'> {
  readonly matching: DeepReadonly<ScannedItem['matching']>;
  scannedItemId: ScannedItemId;
  manualChoice: ManualChoice;
}
