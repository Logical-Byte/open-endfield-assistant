import type { TranslationKey } from '../common';

export interface PrtsAllItemEntry {
  contentId: string;
  desc: TranslationKey;
  firstLvId: string;
  id: string;
  name: TranslationKey;
  order: number;
  overrideRadioId: string;
  type: string;
}

export type PrtsAllItem = Record<string, PrtsAllItemEntry>;
