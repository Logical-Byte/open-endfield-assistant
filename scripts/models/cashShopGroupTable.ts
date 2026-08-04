import type { TranslationKey } from './common';

export interface CashShopGroup {
  cashShopIds: string[];
  icon: string;
  shopGroupId: string;
  shopGroupName: TranslationKey;
  shopGroupType: number;
}

export type CashShopGroupTable = Record<string, CashShopGroup>;
