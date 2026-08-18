import type { TranslationKey } from '../common';

export interface PrtsFirstLvEntry {
  categoryId: string;
  firstLvId: string;
  icon: string;
  itemIds: string[];
  name: TranslationKey;
  order: number;
  subName: TranslationKey;
}

export type PrtsFirstLv = Record<string, PrtsFirstLvEntry>;
