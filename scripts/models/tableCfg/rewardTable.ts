export interface ItemBundle {
  id: string;
  count: number;
}

export interface Reward {
  rewardId: string;
  itemBundles: ItemBundle[];
  probItemBundles: ItemBundle[];
}

export type RewardTable = Record<string, Reward>;
