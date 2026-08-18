import type { TranslationKey } from '../common';

export interface WorldEnergyPointGroup {
  firstPassRewardId: string;
  gameGroupId: string;
  gameGroupName: TranslationKey;
  gemCustomItemId: string;
  gemRandId: string;
  icon: string;
  primAttrTermIds: string[];
  secAttrTermIds: string[];
  skillTermIds: string[];
  worldLevel2GameMechanicsIdMap: Record<string, string>;
}

export type WorldEnergyPointGroupTable = Record<string, WorldEnergyPointGroup>;
