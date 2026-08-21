import type { Vector3 } from '../common';

/** levelScriptBriefDataDict 中的单条关卡脚本简表 */
export interface LevelScriptBrief {
  scriptId: string;
  dataPath: string;
  levelScriptType: string;
  parentLevelScriptId: number;
  maxStage: number;
  properties: unknown;
  propertyIdToKeyMap: unknown;
  /** 脚本引用的关卡实体节点 id 列表（部分脚本才存在） */
  refWorldEntityIdList?: number[];
}

/** 组件属性值（valueArray 元素，脚本读取 valueString / valuestring） */
export interface ComponentPropertyValue {
  valueBit64?: number;
  valueString?: string;
  valuestring?: string;
}

/** 组件属性条目（componentProperties[组件名] 的元素） */
export interface ComponentPropertyEntry {
  key: string;
  value: {
    type: string;
    valueArray: ComponentPropertyValue[];
  };
}

/** 组件属性表：组件名 -> 属性条目列表 */
export type ComponentProperties = Record<string, ComponentPropertyEntry[]>;

/**
 * 关卡实体（interactives / npcs / enemies 等数组元素）。
 * 字段极多，脚本只读取 levelLogicId 与 componentProperties。
 */
export interface LevelEntity {
  levelLogicId: number;
  dependencyGroupId?: string;
  entityType: string;
  entityDataIdKey: string;
  createState: string;
  position: Vector3;
  rotation: Vector3;
  scale: Vector3;
  forceLoad: boolean;
  aoiRadiusType: string;
  overrideSendDieEvent: boolean;
  sendDieEvent: boolean;
  keepCrossMap: boolean;
  isLocked?: boolean;
  isClientOnly?: boolean;
  componentProperties: ComponentProperties;
  [key: string]: unknown;
}

/**
 * Json/LevelData/<scene>/<scene>_lv_data.json。
 * 脚本读取 sceneId、levelScriptBriefDataDict，并遍历实体查找
 * `levelLogicId` + `componentProperties`。
 */
export interface LevelData {
  sceneId: string;
  levelIdNum: number;
  guideHints: unknown[];
  enemies: LevelEntity[];
  interactives: LevelEntity[];
  interactiveLockData: unknown[];
  factoryRegions: unknown[];
  factoryMines: unknown[];
  npcs: LevelEntity[];
  npcClusters: unknown[];
  levelScriptDataPathDict: Record<string, unknown>;
  levelScriptBriefDataDict: Record<string, LevelScriptBrief>;
  patrols: unknown[];
  enemyPatrol: unknown[];
  charPatrol: unknown[];
  npcPatrol: unknown[];
  npcAttractPointData: unknown[];
  missionAreas: unknown[];
  enemyGroup: unknown[];
  npcGroup: unknown[];
  cameraPoses: unknown[];
  splines: unknown[];
  airWalls: unknown[];
  environmentVolumes: unknown[];
  spawners: unknown[];
  worldWayPointData: unknown[];
  waterVolumes: unknown[];
  levelUIs: unknown[];
  aiTransData: unknown[];
  sludgeDatas: unknown[];
  safeZone: unknown;
  factoryPredefineData: unknown;
  predefinedParams: unknown;
  functionArea: unknown;
  doodadGroup: unknown;
  mapVolumeDatas: unknown[];
  riftVolumes: unknown[];
  dynamicOccludeAreas: unknown[];
  autoSpawnedInteractives: unknown[];
  levelWideConfigs: unknown;
}
