import type { Vector3 } from '../common';

/** 世界实体简表（worldEntityBriefInfos 元素） */
export interface WorldEntityBrief {
  entityType: string;
  detailId: string;
  position: Vector3;
  rotation: Vector3;
}

/** NPC 代理简表（npcProxyBriefInfos 元素） */
export interface NpcProxyBrief {
  proxyId: string;
  /** 全局关卡节点 id */
  segmentIdGlobal: number;
  position: Vector3;
}

/**
 * WorldEntityRegistry.json。
 * 脚本只依赖 `npcProxyBriefInfos[proxyId].segmentIdGlobal`。
 */
export interface WorldEntityRegistry {
  worldEntityBriefInfos: Record<string, WorldEntityBrief>;
  npcProxyBriefInfos: Record<string, NpcProxyBrief>;
  m_scriptEntityIdList?: unknown[];
  m_scriptEntityBriefInfo?: Record<string, unknown>;
  worldEntityConfigInfos?: Record<string, unknown>;
  m_npcIdToLogicIdLut?: Record<string, unknown>;
}
