export interface SimulationTrainingConst {
  cardPoolOrder: string[];
  domainId: string;
  doubleRoundLimit: number;
  drawLimit: number;
  foldLimit: number;
  playTimesLimit: number;
  rotationInterval: number;
  simulationTrainingRefLevelId: string;
  startTimeId: string;
  // 其他字段省略，只保留需要的
  [key: string]: unknown;
}
