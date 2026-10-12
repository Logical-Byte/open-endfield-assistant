import type { TranslationKey } from '../common';

export interface BattlePassSeason {
  bannerPresetId: string;
  bussinessCardId: string;
  id: string;
  levelGroupId: string;
  maxLevel: number;
  name: TranslationKey;
  originiumHintRewardId: string;
  originiumPreviewGroupId: string;
  ovrLvRewardGroupId: string;
  payHintRewardId: string;
  payPreviewGroupId: string;
  shortName: TranslationKey;
  weaponBoxId: string;
}

export type BattlePassSeasonTable = Record<string, BattlePassSeason>;
