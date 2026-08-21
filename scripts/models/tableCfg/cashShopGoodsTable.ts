import type { TranslationKey } from '../common';

export interface CashShopGoods {
  cashGoodsId: string;
  cashShopId: string;
  goodsName: TranslationKey;
  goodsType: number;
  iconId: string;
  priceCNY: number;
  priceUSD: number;
  rewardId: string;
}

export type CashShopGoodsTable = Record<string, CashShopGoods>;
