import { readonly, ref, watch, type DeepReadonly, type Ref } from 'vue';
import { whenever } from '@vueuse/core';
import { useAutomationTask, type AutomationTask } from '@/features/automation/useAutomationTask';
import { onScannedItem } from './ipc';
import type { ScannedItem } from './types';

const items = ref<ScannedItem[]>([]);
const error = ref<string | null>(null);
let task: AutomationTask<'essenceScan'> | null = null;

/** 保留整个前端会话中的最近一轮结果，页面切换不会丢失。 */
export const scannedItems: DeepReadonly<Ref<ScannedItem[]>> = readonly(items);
export const scanError: Readonly<Ref<string | null>> = readonly(error);

export function clearScannedItems(): void {
  if (task?.isActive.value) return;
  items.value = [];
}

export async function initScannedItems(): Promise<void> {
  const scanTask = useAutomationTask('essenceScan');
  task = scanTask;
  watch(
    scanTask.runId,
    (runId: number | null) => {
      if (runId === null) return;
      items.value = [];
      error.value = null;
    },
    { flush: 'sync' },
  );
  whenever(
    scanTask.outcome,
    (outcome) => {
      if (outcome.status === 'failed') error.value = outcome.error;
    },
    { immediate: true, flush: 'sync' },
  );
  await onScannedItem(({ runId, payload }) => {
    if (runId !== scanTask.runId.value) return;
    items.value.push(payload);
  });
}
