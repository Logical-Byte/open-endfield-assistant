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
import {
  ArchiveMatchAction,
  type ArchiveMatchChange,
  type ArchiveMatchSnapshot,
} from './types/archiveMatchChange';
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

/** 序列化唯一推荐及其后续匹配变更，无需记录时返回 null。 */
function serializeArchiveMatchChange(
  item: Readonly<ScannedItemRecord>,
  action: ArchiveMatchAction,
  previous: Readonly<ScannedItemRecord> | null,
): string | null {
  if (item.matching.kind !== 'recommendations' || item.matching.archiveIds.length !== 1)
    return null;
  const before = previous ? matchedArchiveId(previous) : null;
  const after = matchedArchiveId(item);
  if (previous && before === after) return null;

  function describeArchive(id: ArchiveId | null): ArchiveMatchSnapshot | null {
    if (id === null) return null;
    const archive = archiveCatalog.value?.archive(id);
    return {
      id,
      gameLocale: item.gameLocale,
      title: archive?.title[item.gameLocale] ?? null,
      contentPreview: archive?.contentPreview?.[item.gameLocale] ?? null,
    };
  }
  const details: ArchiveMatchChange = {
    scan: {
      scannedItemId: item.scannedItemId,
      gameLocale: item.gameLocale,
      foundInPage: item.foundInPage,
      foundInCategory: item.foundInCategory,
      ocrResult: item.ocrResult,
      bodyOcrResult: item.bodyOcrResult ?? null,
      matching: item.matching,
    },
    action,
    before: describeArchive(before),
    after: describeArchive(after),
  };
  return JSON.stringify(details);
}

/** 只在接收或编辑时发送日志，派生匹配和筛选不产生副作用。 */
function logArchiveMatchChange(
  item: Readonly<ScannedItemRecord>,
  action: ArchiveMatchAction,
  previous: Readonly<ScannedItemRecord> | null,
): void {
  const message = serializeArchiveMatchChange(item, action, previous);
  if (message === null) return;
  const descriptions: Record<ArchiveMatchAction, string> = {
    [ArchiveMatchAction.AutoAdopt]: '系统自动采纳了唯一推荐。',
    [ArchiveMatchAction.ManualSelect]: '用户改选了匹配档案。',
    [ArchiveMatchAction.ManualClear]: '用户清除了档案匹配。',
    [ArchiveMatchAction.Undo]: '用户撤销了匹配编辑。',
  };
  void logInfo(`${descriptions[action]} ${message}`).catch((error: unknown) => {
    console.warn('写入档案匹配变更日志失败', error);
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
  logArchiveMatchChange(
    next,
    archiveId === null ? ArchiveMatchAction.ManualClear : ArchiveMatchAction.ManualSelect,
    item,
  );
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
  logArchiveMatchChange(next, ArchiveMatchAction.Undo, item);
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
    // 自动采纳前尚未建立前端匹配。
    logArchiveMatchChange(record, ArchiveMatchAction.AutoAdopt, null);
  });
}
