export interface GiftpackCashShopGoodsData {
  anchorCashGoodsId: string;
  availCount: number;
  availRefresh: number;
  bg: string;
  bigBg: string;
  bigIcon: string;
  cashGoodsId: string;
  clientShowAfterGoodsId: string;
  conditionOpType: number;
  deco: string;
  dontShowWhenSellOut: boolean;
  dynamicPriority: number;
  dynamicTag: boolean;
  hideInGame: boolean;
  priority: number;
  tagList: string[];
}

export type GiftpackCashShopGoodsDataTable = Record<string, GiftpackCashShopGoodsData>;
