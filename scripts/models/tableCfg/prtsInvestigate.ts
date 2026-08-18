import type { TranslationKey } from '../common';

/** 调查中的一个研究阶段（categoryDataList 元素） */
export interface PrtsInvestigateCategoryData {
  /** 本阶段解锁的档案 id 列表 */
  collectionIdList: string[];
  /** 阶段序号 */
  index: number;
  /** 阶段名 */
  name: TranslationKey;
  /** 本阶段笔记 id 列表 */
  noteIdList: string[];
}

/** PrtsInvestigate.json 中的单条研究记录 */
export interface PrtsInvestigateEntry {
  /** 各阶段数据（按阶段推进解锁档案） */
  categoryDataList: PrtsInvestigateCategoryData[];
  /** 整个研究累计解锁的档案 id 列表 */
  collectionIdList: string[];
  desc: TranslationKey;
  /** 研究所属领域 id（如 domain_2） */
  domainId: string;
  /** 研究 id（如 research_001） */
  id: string;
  /** 研究序号 */
  index: number;
  investigateAreaDesc: TranslationKey;
  name: TranslationKey;
  /** 完成研究后发放的奖励表 id */
  rewardId: string;
  /** 研究类型 */
  type: number;
  /** 研究最终解锁的档案 id */
  unlockPrts: string;
}

/** PrtsInvestigate.json：researchId -> 研究记录 */
export type PrtsInvestigate = Record<string, PrtsInvestigateEntry>;
