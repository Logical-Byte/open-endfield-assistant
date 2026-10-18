import type { ArchiveId } from '@/shared/types/archive';
import type { GameLocale } from '@/shared/types/locale';
import type { ScannedItemRecord } from './scannedItem';

/** 稳定的动作值，供日志及后续遥测消费。 */
export enum ArchiveMatchAction {
  AutoAdopt = 'autoAdopt',
  ManualSelect = 'manualSelect',
  ManualClear = 'manualClear',
  Undo = 'undo',
}

/** 变更发生时、扫描语言下的档案信息。 */
export interface ArchiveMatchSnapshot {
  readonly id: ArchiveId;
  readonly gameLocale: GameLocale;
  readonly title: string | null;
  readonly contentPreview: string | null;
}

/** 匹配变更的可序列化记录，独立于日志或遥测的传输方式。 */
export interface ArchiveMatchChange {
  readonly scan: Pick<
    Readonly<ScannedItemRecord>,
    'scannedItemId' | 'gameLocale' | 'foundInPage' | 'foundInCategory' | 'ocrResult' | 'matching'
  > & { readonly bodyOcrResult: string | null };
  readonly action: ArchiveMatchAction;
  readonly before: ArchiveMatchSnapshot | null;
  readonly after: ArchiveMatchSnapshot | null;
}
