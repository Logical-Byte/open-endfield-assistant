import type { Vector3 } from '../common';

/** 组件引用（{tagId} / {key}） */
export interface NpcProxyTagRef {
  tagId?: number;
  key?: string;
}

/**
 * NpcProxyTable.json dataTable 中的单条 NPC 代理记录。
 *
 * 字段众多（120+），这里仅列出从数据中确认的主要字段；
 * 脚本只依赖 `dataTable[proxyId]` 的存在性检查。
 */
export interface NpcProxy {
  subDataParentId: number;
  /** 关卡节点 logicId */
  levelLogicId: number;
  entityType: string;
  createState: string;
  position: Vector3;
  rotation: Vector3;
  scale: Vector3;
  forceLoad: boolean;
  aoiRadiusType: string;
  overrideSendDieEvent: boolean;
  sendDieEvent: boolean;
  keepCrossMap: boolean;
  npcGroupId: string;
  type: number;
  doPatrol: boolean;
  defaultActivePatrol: boolean;
  patrolCfgType: string;
  initPatrolIndex: number;
  patrolId_New: number;
  defaultMontage: NpcProxyTagRef;
  overrideMontageState: boolean;
  montageState: string;
  autoPreloadMontages: boolean;
  preloadMontages: unknown[];
  defaultMontageMaskType: string;
  collisionEnable: boolean;
  overrideInteractRange: boolean;
  interactRangeType: string;
  disableEmotion: boolean;
  defaultEmotion: NpcProxyTagRef;
  defaultFacialAnim: NpcProxyTagRef;
  lookAt: boolean;
  enableDialogLookAtCapability: boolean;
  doStim: boolean;
  stimulateKey: string;
  atmosphereStimulateCanMove: boolean;
  isOverriderBlur: boolean;
  needBlurCheck: boolean;
  blurPriority: number;
  confrontAnim: NpcProxyTagRef;
  battleAnim: NpcProxyTagRef;
  idleBreakTags: unknown[];
  overrideConfrontRot: boolean;
  needConfrontRot: boolean;
  overrideBattleRot: boolean;
  needBattleRot: boolean;
  ignoreBattleReturn: boolean;
  hideHeadLabel: boolean;
  hideHeadName: boolean;
  aiCfg: string;
  belongStoryZoneId: number;
  ifOverrideNpcName: boolean;
  overrideNpcNameId: NpcProxyTagRef;
  ifOverrideTitle: boolean;
  overrideNpcTitleId: NpcProxyTagRef;
  ifOverrideFaction: boolean;
  overrideNpcFactionId: NpcProxyTagRef;
  envTalkIds: string[];
  envTalkOdd: number[];
  hitData: unknown;
  notifyInteractEvent: boolean;
  controlByLevelScript: boolean;
  overrideDefaultInteractText: boolean;
  overrideDefaultInteractIcon: boolean;
  defaultInteractText: unknown;
  interactionIcon: unknown;
  envTalkTriggerDistance: number;
  envTalkOverrideNpc: boolean;
  disableDowngrade: boolean;
  enableMorph: boolean;
  enableCloth: boolean;
  overrideSpIdleConfig: boolean;
  enableDownGradeSpIdle: boolean;
  normalIdle2SpidleTime: number;
  spIdle2normalIdleTime: number;
  spIdleRandomWaitTimeMin: number;
  spIdleRandomWaitTimeMax: number;
  npcPatrolGroupId: string;
  battleDataOverride: unknown;
  linkedChairId: unknown;
  /** NPC 代理 id */
  proxyId: string;
  /** 所属关卡 id */
  levelId: string;
  overrideLabel: boolean;
  hideBubble: boolean;
  clusterId: string;
  hidePopupExpression: boolean;
  ifOverrideHeadIcon: boolean;
  overrideHeadIcon: unknown;
  needWayPoint: boolean;
  overrideTemplateAI: boolean;
  overrideAbilitySo: boolean;
  lazyDestroy: boolean;
  lazyDestroyEnvTalkData: unknown;
  lazyDestroyOverrideDialogId: unknown;
  lazyDestroyStartPatrol: boolean;
  lazyDestroyPatrolId: unknown;
  overrideTemplateAi: boolean;
  overrideGamePlayData: boolean;
  overrideAudio: boolean;
  overrideInitAudioId: unknown;
}

/** NpcProxyTable.json：{ dataTable: proxyId -> NPC 代理 } */
export interface NpcProxyTable {
  dataTable: Record<string, NpcProxy>;
}
