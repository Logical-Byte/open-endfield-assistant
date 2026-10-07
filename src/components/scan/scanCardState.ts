import type { ScannedItemView } from '@/features/archiveScan/resultView';

/** 放在虚拟列表之外，滚动卸载卡片时保留草稿和折叠状态。 */
export interface ScanCardState {
  expanded: boolean | undefined;
  editing: boolean;
  draft: string;
  showOcr: boolean;
}
export function createScanCardState(item: ScannedItemView): ScanCardState {
  return {
    expanded: undefined,
    editing: false,
    draft: item.correctedTitle ?? item.ocrResult,
    showOcr: false,
  };
}
