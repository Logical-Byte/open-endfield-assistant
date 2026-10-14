import type { ScannedItemView } from '@/features/archiveScan/resultView';
import type { ArchiveId } from '@/shared/types/archive';

/** 放在虚拟列表之外，滚动卸载卡片时保留草稿和折叠状态。 */
export interface ScanCardState {
  expanded: boolean | undefined;
  editing: boolean;
  draftArchiveId: ArchiveId | null;
  draftTouched: boolean;
  search: string;
  showOcr: boolean;
}
export function createScanCardState(item: ScannedItemView): ScanCardState {
  return {
    expanded: undefined,
    editing: false,
    draftArchiveId: item.archive?.id ?? null,
    draftTouched: false,
    search: '',
    showOcr: false,
  };
}
