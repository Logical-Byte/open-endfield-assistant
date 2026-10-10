import type {
  ScannedItem,
  ScannedItemRecord,
  ScannedItemId,
} from '@/features/archiveScan/types/scannedItem';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import type { RunOutcome } from '@/features/automation/types';
import { archiveCatalog } from '@/features/gameData/archiveCatalog';
import { onScannedItem } from './ipc';
import { normalizeBackendError, type ErrorFacts } from '@/shared/errors';
import { whenever } from '@vueuse/core';
import { readonly, ref, type DeepReadonly, type Ref } from 'vue';

const items: Ref<ScannedItemRecord[]> = ref([]);
// 清空后继续递增，旧卡片的纠错操作不会命中新扫描的记录。
let nextScannedItemId: number = 1;

/** 深度只读的扫描记录，随扫描进度追加，通过 ID 提交人工纠错。 */
export const scannedItems: DeepReadonly<Ref<ScannedItemRecord[]>> = readonly(items);

/** 最近一次档案扫描失败的原因，新扫描启动时清除。 */
const error: Ref<ErrorFacts | null> = ref(null);
export const scanError: Readonly<Ref<ErrorFacts | null>> = readonly(error);

/** 清空扫描结果列表。 */
export function clearScannedItems(): void {
  items.value = [];
}

/** 在扫描时所在的小分类内匹配标题，一次替换记录的纠错字段。 */
export function correctScannedItem(scannedItemId: ScannedItemId, title: string): void {
  const index = items.value.findIndex(
    (item: ScannedItemRecord): boolean => item.scannedItemId === scannedItemId,
  );
  // 卡片可能在清空或重新扫描后才提交编辑。
  if (index === -1) return;

  const item = items.value[index]!;
  const correctedMatchItemIds = archiveCatalog.value?.idsByTitle(item.foundInCategory, title) ?? [];
  items.value[index] = {
    ...item,
    correctedTitle: title,
    manuallyCorrected: true,
    status: correctedMatchItemIds.length > 0 ? 'success' : 'unrecognized',
    correctedMatchItemIds,
  };
}

/** 撤销人工纠错，只恢复纠错字段，保留记录身份和原始截图/OCR。 */
export function restoreScannedItemCorrection(previous: Readonly<ScannedItemRecord>): void {
  const index = items.value.findIndex(
    (item: ScannedItemRecord): boolean => item.scannedItemId === previous.scannedItemId,
  );
  if (index === -1) return;
  items.value[index] = {
    ...items.value[index]!,
    correctedTitle: previous.correctedTitle,
    manuallyCorrected: previous.manuallyCorrected,
    correctedMatchItemIds: previous.correctedMatchItemIds,
    status: previous.status,
  };
}

export async function initScannedItems(): Promise<void> {
  const task = useAutomationTask('archiveScan');
  whenever(
    (): boolean => task.phase.value === 'running',
    () => {
      clearScannedItems();
      error.value = null;
    },
    { flush: 'sync' },
  );
  whenever(
    task.outcome,
    (outcome: Readonly<RunOutcome>) => {
      if (outcome.status === 'failed')
        error.value = normalizeBackendError(outcome.error, 'archiveScan');
    },
    { flush: 'sync' },
  );
  await onScannedItem((item: ScannedItem) => {
    items.value.push({
      ...item,
      scannedItemId: nextScannedItemId++ as ScannedItemId,
      manuallyCorrected: false,
    });
  });
}
