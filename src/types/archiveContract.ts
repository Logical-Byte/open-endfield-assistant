/** 档案分类 id（archive_contract.json categories 的键） */
export const ARCHIVE_CATEGORY_IDS = [
  'paper',
  'digital',
  'collection',
  'document',
  'report',
  'media',
] as const;
export type ArchiveCategoryId = (typeof ARCHIVE_CATEGORY_IDS)[number];

/** 获取方式（acquisition.method 判别字段） */
export type ArchiveAcquisitionMethod = 'map' | 'mission' | 'auto' | 'shop' | 'invstgt';

/** 地图交互点位获取（method = 'map'） */
export interface ArchiveMapAcquisition {
  method: 'map';
  pointId: number;
}

/** 任务内关卡交互信息（mission.interaction） */
export interface ArchiveMissionInteraction {
  pointId: number;
  /** 任务阶段文本 id */
  stageId: string;
}

/** 任务特殊信息（mission.special，NPC 对话 + Baker 消息链） */
export interface ArchiveMissionSpecial {
  npcId: string;
  dialogOptionId: string;
  bakerChatId: string;
  bakerDialogId: string;
}

/** 完成任务获取（method = 'mission'） */
export interface ArchiveMissionAcquisition {
  method: 'mission';
  missionId: string;
  questId?: string;
  interaction?: ArchiveMissionInteraction;
  special?: ArchiveMissionSpecial;
}

/** 自动解锁（method = 'auto'），无附加字段 */
export interface ArchiveAutoAcquisition {
  method: 'auto';
}

/** 商店位置（shop.location） */
export interface ArchiveShopLocation {
  regionId: string;
  subregionId?: string;
}

/** 商店兑换获取（method = 'shop'） */
export interface ArchiveShopAcquisition {
  method: 'shop';
  shopGroupId: string;
  shopId: string;
  npcId?: string;
  pointId?: number;
  location?: ArchiveShopLocation;
}

/** 研究提交获取（method = 'invstgt'） */
export interface ArchiveInvestigateAcquisition {
  method: 'invstgt';
  researchId: string;
}

/** 档案获取方式（以 method 判别的联合类型） */
export type ArchiveAcquisition =
  | ArchiveMapAcquisition
  | ArchiveMissionAcquisition
  | ArchiveAutoAcquisition
  | ArchiveShopAcquisition
  | ArchiveInvestigateAcquisition;

/** archive_contract.json 中的单条档案 */
export interface ArchiveContractRow {
  /** 档案条目 id */
  id: string;
  /** 档案图标 */
  icon: string;
  acquisition: ArchiveAcquisition;
}

/**
 * resources/data/archive_contract.json（makeAllData 的 exportArchiveContract 输出）
 * 的运行时契约，供前端按档案 id 查询获取方式。
 */
export interface ArchiveContract {
  version: 1;
  categories: Record<ArchiveCategoryId, ArchiveContractRow[]>;
}
