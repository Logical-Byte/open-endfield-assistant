import { readonly, shallowRef } from 'vue';
import type {
  EssenceScanReport,
  EssenceScanRecord,
  EssenceScanProgress,
} from '@/shared/types/essenceScan';
import { getEssenceScanReport, onEssenceItemScanned, onEssenceScanProgress } from './ipc';

// 应用级投影，切换页面后仍保留。初始化快照来自后端，后续按 runId 合并事件。
const report = shallowRef<EssenceScanReport | null>(null);
export const essenceScanReport = readonly(report);

function receiveProgress(progress: EssenceScanProgress): void {
  const previous = report.value;
  if (previous && progress.runId < previous.progress.runId) return;
  report.value = {
    progress,
    records: previous?.progress.runId === progress.runId ? previous.records : [],
  };
}

function receiveRecord(record: EssenceScanRecord): void {
  const current = report.value;
  if (!current || record.runId !== current.progress.runId) return;
  report.value = { ...current, records: [...current.records, record] };
}

export async function initEssenceScan(): Promise<void> {
  report.value = await getEssenceScanReport();
  await onEssenceScanProgress(receiveProgress);
  await onEssenceItemScanned(receiveRecord);
}

/** 浏览器预览专用的演示数据，使用相同 contract，页面始终标注模拟来源。 */
export async function simulateEssenceScanInBrowser(): Promise<void> {
  const runId = (report.value?.progress.runId ?? 0) + 1;
  receiveProgress({
    runId,
    workerType: 'simulation',
    phase: 'scanning',
    scannedCount: 0,
    incompleteCount: 0,
    outcome: null,
  });
  for (let index = 0; index < 12; index++) {
    await new Promise<void>((resolve) => setTimeout(resolve, 150));
    const current = report.value;
    if (!current || current.progress.runId !== runId || current.progress.outcome) return;
    const record: EssenceScanRecord = {
      runId,
      recordId: index + 1,
      row: Math.floor(index / 9) + 1,
      column: (index % 9) + 1,
      terms: [
        { termId: 'gat_passive_attr_str', kind: 'attribute', level: 3 },
        { termId: 'gat_passive_attr_atk', kind: 'secondary', level: 3 },
        { termId: 'gst_passive_tactic', kind: 'skill', level: 3 },
      ],
      rarity: index < 6 ? 'five' : 'four',
      locked: index >= 2 && index % 2 === 0,
      abandoned: index % 3 === 2,
      issues: [],
      image: '',
    };
    if (index === 4) {
      record.terms[2] = { termId: null, kind: null, level: null };
      record.locked = null;
      record.issues = [
        { kind: 'term', position: 3 },
        { kind: 'level', position: 3 },
        { kind: 'locked' },
      ];
    }
    receiveRecord(record);
    receiveProgress({
      ...current.progress,
      scannedCount: index + 1,
      incompleteCount: current.progress.incompleteCount + (record.issues.length > 0 ? 1 : 0),
    });
  }
  const current = report.value;
  if (current?.progress.runId === runId)
    receiveProgress({ ...current.progress, phase: 'finished', outcome: { kind: 'completed' } });
}

export function stopEssenceScanInBrowser(): void {
  const current = report.value;
  if (current)
    receiveProgress({ ...current.progress, phase: 'finished', outcome: { kind: 'stopped' } });
}
