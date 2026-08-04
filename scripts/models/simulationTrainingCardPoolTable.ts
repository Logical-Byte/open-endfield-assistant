export interface SimulationTrainingCard {
  cardNum: number;
  cardPoolID: string;
  cardWeight: number;
  enemyGroupId: string;
}

export interface SimulationTrainingCardPool {
  list: SimulationTrainingCard[];
}

export interface SimulationTrainingCardPoolTable {
  [poolId: string]: SimulationTrainingCardPool;
}
