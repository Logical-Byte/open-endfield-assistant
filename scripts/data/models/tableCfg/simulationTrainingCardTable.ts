export interface SimulationTrainingCard {
  cardPoint: number;
  enemyCountList: number[];
  enemyGroupId: string;
  enemyIdList: string[];
  enemyLevel: number[];
  isBonusCard: boolean;
}

export interface SimulationTrainingCardTable {
  [enemyGroupId: string]: SimulationTrainingCard;
}
