export interface BattlePassLevel {
  buyHintType: number;
  freeRewardId: string;
  isMilestone: boolean;
  isRecurring: boolean;
  level: number;
  levelExp: number;
  originiumRewardId: string;
  payRewardId: string;
}

export interface BattlePassLevelGroup {
  levelGroupId: string;
  levelInfos: Record<string, BattlePassLevel>;
}

export type BattlePassLevelTable = Record<string, BattlePassLevelGroup>;
