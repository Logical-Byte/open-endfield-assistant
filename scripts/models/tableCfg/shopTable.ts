import type { TranslationKey } from '../common';

/** ShopTable.json 中的单个商店 */
export interface Shop {
  iconId: string;
  isShowWhenLock: boolean;
  lockDesc: TranslationKey;
  shopEnName: TranslationKey;
  /** 商店内商品 id 列表 */
  shopGoodsIds: string[];
  /** 商店组 id（用于关联商店位置 / NPC） */
  shopGroupId: string;
  shopGroupNumber: number;
  /** 商店 id */
  shopId: string;
  shopName: TranslationKey;
  shopRefreshCycleType: number;
  shopRefreshType: number;
  unlockConditions: unknown[];
}

/** ShopTable.json：shopId -> 商店 */
export type ShopTable = Record<string, Shop>;
