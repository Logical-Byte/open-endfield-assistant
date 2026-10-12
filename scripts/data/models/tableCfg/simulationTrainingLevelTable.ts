import type { TranslationKey } from '../common';

export interface SimulationTrainingLevel {
  costDomainMoney: number;
  desc: TranslationKey;
  domainDevExp: number;
  doubleLimit: number;
  gamblingBattleLevel: number;
  isFinalMaxLevel: boolean;
  pointAward: number[];
}

export interface SimulationTrainingLevelTable {
  [level: string]: SimulationTrainingLevel;
}
