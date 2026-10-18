import type {
  ScannedItem,
  ScannedItemRecord,
  ScannedItemId,
} from '@/features/archiveScan/types/scannedItem';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import type { RunOutcome } from '@/features/automation/types';
import type { ArchiveId } from '@/shared/types/archive';
import { archiveCatalog } from '@/features/gameData/archiveCatalog';
import { logInfo } from '@/features/log/ipc';
import { matchedArchiveId } from './matching';
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

/** 只在接收或编辑时记录，派生匹配和筛选不产生副作用。 */
function logSingleRecommendation(
  item: Readonly<ScannedItemRecord>,
  action: 'automatic' | 'manual' | 'undo',
): void {
  if (item.matching.kind !== 'recommendations' || item.matching.archiveIds.length !== 1) return;
  const adoptedArchiveId = item.matching.archiveIds[0]!;
  const details = {
    action,
    scannedItemId: item.scannedItemId,
    adoptedArchiveId,
    category: item.foundInCategory,
    ocrResult: item.ocrResult,
    title: archiveCatalog.value?.archive(adoptedArchiveId)?.title[item.gameLocale] ?? null,
    archiveId: matchedArchiveId(item),
    manualChoice: item.manualChoice,
  };
  const message = action === 'automatic' ? '单个推荐已自动采纳' : '单个推荐采纳后的人工决定已变更';
  // JSON 编码保留原始文字，同时避免 OCR 换行打断日志记录。
  void logInfo(`${message}：${JSON.stringify(details)}`).catch((error: unknown) => {
    console.warn('写入单个推荐采纳日志失败', error);
  });
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
  const next: ScannedItemRecord = { ...item, manualChoice: { kind: 'manual', archiveId } };
  items.value[index] = next;
  if (matchedArchiveId(item) !== archiveId) logSingleRecommendation(next, 'manual');
}

/** 撤销一次已确认编辑，只恢复人工决定，保留扫描观察与自动结果。 */
export function restoreScannedItemChoice(previous: Readonly<ScannedItemRecord>): void {
  const index = items.value.findIndex(
    (item): boolean => item.scannedItemId === previous.scannedItemId,
  );
  if (index === -1) return;
  const item = items.value[index]!;
  const next: ScannedItemRecord = { ...item, manualChoice: previous.manualChoice };
  items.value[index] = next;
  if (matchedArchiveId(item) !== matchedArchiveId(next)) logSingleRecommendation(next, 'undo');
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
    const record: ScannedItemRecord = {
      ...item,
      scannedItemId: nextScannedItemId++ as ScannedItemId,
      manualChoice: { kind: 'automatic' },
    };
    items.value.push(record);
    logSingleRecommendation(record, 'automatic');
  });
}
