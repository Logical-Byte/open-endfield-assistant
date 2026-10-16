import type { TranslationKey } from '../common';

/** ShopGoodsTable.json 中的单条商品 */
export interface ShopGoods {
  /** 折扣（1.0 = 无折扣） */
  cnDiscount: number;
  /** 商品 id */
  goodsId: string;
  /** 商品标签 id */
  goodsTagId: string;
  isShowWhenLock: boolean;
  /** 限购数量（0 = 不限） */
  limitCount: number;
  limitCountRefreshType: number;
  lockDesc: TranslationKey;
  /** 购买货币 item id */
  moneyId: string;
  /** 价格 */
  price: number;
  randomGoodsStandardPrice: number;
  relatedWeaponGachPoolId: string;
  /** 购买后发放的奖励表 id（archive 通过该奖励表获得） */
  rewardId: string;
  /** 所属商店 id */
  shopId: string;
  sortId: number;
  unlockConditions: unknown[];
  weaponGachaPoolId: string;
}

/** ShopGoodsTable.json：goodsId -> 商品 */
export type ShopGoodsTable = Record<string, ShopGoods>;
