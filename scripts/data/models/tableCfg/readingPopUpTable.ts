import type { TranslationKey } from '../common';

/** ReadingPopUpTable.json 中的单条阅读弹窗 */
export interface ReadingPopUp {
  bgType: number;
  /** 阅读内容 id（关联 PrtsAllItem 的 contentId） */
  contentId: string;
  iconType: number;
  /** 弹窗 id */
  id: string;
  overrideRadioId: string;
  title: TranslationKey;
}

/** ReadingPopUpTable.json：readingId -> 阅读弹窗 */
export type ReadingPopUpTable = Record<string, ReadingPopUp>;
