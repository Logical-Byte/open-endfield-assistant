import type { TranslationKey } from './common';

export interface CashShop {
  cashGoodsIds: string[];
  cashShopId: string;
  shopName: TranslationKey;
}

export type CashShopTable = Record<string, CashShop>;
