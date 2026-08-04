import type { TranslationKey } from './common';

export interface WorldEnergyPoint {
  costStamina: number;
  desc: TranslationKey;
  enemyIds: string[];
  enemyLevels: number[];
  gameCategory: string;
  gameGroupId: string;
  gameMechanicsId: string;
  gameName: TranslationKey;
  levelId: string;
  probGemItemIds: string[];
  recommendLv: number;
  regularItemCount: unknown[];
  regularItemIds: unknown[];
  worldLevel: number;
}

export type WorldEnergyPointTable = Record<string, WorldEnergyPoint>;
