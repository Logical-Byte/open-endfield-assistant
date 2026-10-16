import type {
  ScannedItem,
  ScannedItemRecord,
  ScannedItemId,
} from '@/features/archiveScan/types/scannedItem';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import type { RunOutcome } from '@/features/automation/types';
import type { ArchiveId } from '@/shared/types/archive';
import { archiveCatalog } from '@/features/gameData/archiveCatalog';
import { onScannedItem } from './ipc';
import { normalizeBackendError, type ErrorFacts } from '@/shared/errors';
import { whenever } from '@vueuse/core';
import { readonly, ref, type DeepReadonly, type Ref } from 'vue';

const items: Ref<ScannedItemRecord[]> = ref([]);
// 清空后继续递增，旧卡片的选择操作不会命中新扫描的记录。
let nextScannedItemId: number = 1;

/** 深度只读的扫描记录，随扫描进度追加，通过 ID 提交档案选择。 */
export const scannedItems: DeepReadonly<Ref<ScannedItemRecord[]>> = readonly(items);

/** 最近一次档案扫描失败的原因，新扫描启动时清除。 */
const error: Ref<ErrorFacts | null> = ref(null);
export const scanError: Readonly<Ref<ErrorFacts | null>> = readonly(error);

/** 清空扫描结果列表。 */
export function clearScannedItems(): void {
  items.value = [];
}

/** 选择扫描分类内的具体档案，空值表示用户明确保留为未匹配。 */
export function chooseScannedItemArchive(
  scannedItemId: ScannedItemId,
  archiveId: ArchiveId | null,
): void {
  const index = items.value.findIndex(
    (item: ScannedItemRecord): boolean => item.scannedItemId === scannedItemId,
  );
  if (index === -1) return;
  const item = items.value[index]!;
  if (archiveId && archiveCatalog.value?.archive(archiveId)?.category !== item.foundInCategory)
    return;
  items.value[index] = { ...item, manualChoice: { kind: 'manual', archiveId } };
}

/** 撤销一次已确认编辑，只恢复人工决定，保留扫描观察与自动结果。 */
export function restoreScannedItemChoice(previous: Readonly<ScannedItemRecord>): void {
  const index = items.value.findIndex(
    (item): boolean => item.scannedItemId === previous.scannedItemId,
  );
  if (index === -1) return;
  items.value[index] = { ...items.value[index]!, manualChoice: previous.manualChoice };
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
      if (
        outcome.status === 'failed' &&
        !(
          outcome.error.scope === 'worker' &&
          outcome.error.kind === 'capability' &&
          outcome.error.reason.kind === 'stoppedByUser'
        )
      )
        error.value = normalizeBackendError(outcome.error, { operation: 'automation' });
    },
    { flush: 'sync' },
  );
  await onScannedItem((item: ScannedItem) => {
    items.value.push({
      ...item,
      scannedItemId: nextScannedItemId++ as ScannedItemId,
      manualChoice: { kind: 'automatic' },
    });
  });
}
