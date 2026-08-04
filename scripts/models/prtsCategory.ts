import type { TranslationKey } from './common';

export interface PrtsCategoryEntry {
  categoryId: string;
  name: TranslationKey;
  order: number;
  tabIcon: string;
}

export type PrtsCategory = Record<string, PrtsCategoryEntry>;
