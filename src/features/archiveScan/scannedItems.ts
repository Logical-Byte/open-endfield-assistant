import type { ScannedItem } from '@/features/archiveScan/types/scannedItem';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import type { RunOutcome } from '@/features/automation/types';
import { getItemIdsByTitle } from '@/features/gameData/archiveQueries';
import { onScannedItem } from './ipc';
import { whenever } from '@vueuse/core';
import { readonly, ref, type DeepReadonly, type Ref } from 'vue';

/** 前端会话内的一条扫描记录，ID 与档案 ID 无关。 */
export interface ScannedItemRecord extends ScannedItem {
  scannedItemId: number;
}

const items: Ref<ScannedItemRecord[]> = ref([]);
// 清空后继续递增，旧卡片的纠错操作不会命中新扫描的记录。
let nextScannedItemId: number = 1;

/** 深度只读的扫描记录，随扫描进度追加，通过 ID 提交人工纠错。 */
export const scannedItems: DeepReadonly<Ref<ScannedItemRecord[]>> = readonly(items);

/** 最近一次档案扫描失败的原因，新扫描启动时清除。 */
const error: Ref<string | null> = ref(null);
export const scanError: Readonly<Ref<string | null>> = readonly(error);

/** 清空扫描结果列表。 */
export function clearScannedItems(): void {
  items.value = [];
}

/** 在扫描时所在的小分类内匹配标题，一次替换记录的纠错字段。 */
export function correctScannedItem(scannedItemId: number, title: string): void {
  const index = items.value.findIndex(
    (item: ScannedItemRecord): boolean => item.scannedItemId === scannedItemId,
  );
  // 卡片可能在清空或重新扫描后才提交编辑。
  if (index === -1) return;

  const item = items.value[index]!;
  const correctedMatchItemIds = getItemIdsByTitle(item.foundInSubCategory, title);
  items.value[index] = {
    ...item,
    correctedTitle: title,
    status: correctedMatchItemIds.length > 0 ? 'success' : 'unrecognized',
    correctedMatchItemIds,
  };
}

export async function initScannedItems(): Promise<void> {
  const task = useAutomationTask('archiveScan');
  whenever(
    task.runId,
    () => {
      clearScannedItems();
      error.value = null;
    },
    { flush: 'sync' },
  );
  whenever(
    task.outcome,
    (outcome: Readonly<RunOutcome>) => {
      if (outcome.status === 'failed') error.value = outcome.error;
    },
    { flush: 'sync' },
  );
  await onScannedItem(({ runId, payload }) => {
    if (runId !== task.runId.value) return;
    items.value.push({ ...payload, scannedItemId: nextScannedItemId++ });
  });
}
