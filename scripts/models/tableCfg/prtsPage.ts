import type { TranslationKey } from '../common';

export interface PrtsPageEntry {
  icon: string;
  name: TranslationKey;
  pageType: string;
}

export type PrtsPage = Record<string, PrtsPageEntry>;
