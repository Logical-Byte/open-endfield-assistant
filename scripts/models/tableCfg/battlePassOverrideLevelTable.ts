export interface BattlePassOverrideLevel {
  freeRewardId: string;
  level: number;
  originiumRewardId: string;
  payRewardId: string;
}

export interface BattlePassOverrideLevelGroup {
  levelGroupId: string;
  levelInfos: Record<string, BattlePassOverrideLevel>;
}

export type BattlePassOverrideLevelTable = Record<string, BattlePassOverrideLevelGroup>;
