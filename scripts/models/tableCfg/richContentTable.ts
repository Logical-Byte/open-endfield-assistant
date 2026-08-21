import type { TranslationKey } from '../common';

export interface RichContentItem {
  content: TranslationKey;
}

export interface RichContent {
  contentList: RichContentItem[];
  title: TranslationKey;
}

export type RichContentTable = Record<string, RichContent>;
