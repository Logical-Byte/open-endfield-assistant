/**
 * 档案获取契约（resources/data/archive_acquisition_contract.json）的类型定义。
 *
 * 该数据文件描述档案库全部 462 个条目（对应 prts.json allItems 的 id）的获取方式。
 * 每条记录以 `method` 为判别字段，对应不同的获取途径与附加字段：
 *
 *   mission（完成任务）  → missionId / questId / logicId / special
 *   map（地图交互点位）  → logicId
 *   spec（特殊交互）     → special（levelId + logicId）
 *   auto（自动解锁）     → 无附加字段
 *   shop（商店兑换）     → shopGroupId / shopId / npcId / logicIds
 *   invstgt（研究提交）  → researchId
 *
 * 数据结构为「以 method 判别的联合类型」：公共字段 type / method 见
 * ArchiveAcquisitionContractBase，各变体通过字面量收窄 method 并携带各自的附加字段。
 */

/** 档案获取方式（判别字段 method） */
export type AcquisitionMethod = 'mission' | 'map' | 'spec' | 'auto' | 'shop' | 'invstgt';

/**
 * 单条档案获取契约的公共字段。
 * 各变体（MissionAcquisition 等）继承本接口并以字面量收窄 `method`。
 */
export interface ArchiveAcquisitionContractBase {
  /** 档案条目 id（对应 prts.json allItems 的 id） */
  type: string;
  /** 获取方式 */
  method: AcquisitionMethod;
}

/** 任务类获取的对话特殊信息（mission.special，NPC 对话 + Baker 消息链） */
export interface MissionSpecial {
  /** NPC id（如 suosi_map01） */
  npcId: string;
  /** NPC 对话选项 id（如 option_dlg_map01_lv002_env_8_1_001） */
  dialogOptionId: string;
  /** Baker 消息链 id（如 sns_chat_nfm_0_1） */
  bakerChatId: string;
  /** Baker 对话 id（如 sns_f1m4d1_4） */
  bakerDialogId: string;
}

/** 特殊交互类获取的关卡节点信息（spec.special） */
export interface SpecSpecial {
  /** 关卡 id（如 map01_lv001） */
  levelId: string;
  /** 关卡内节点 logicId */
  logicId: number;
}

/** 通过完成任务获得（method = 'mission'） */
export interface MissionAcquisition extends ArchiveAcquisitionContractBase {
  method: 'mission';
  /** 任务 id */
  missionId: string;
  /** 任务内子任务（quest）id */
  questId?: string;
  /** 任务内节点 logicId */
  logicId?: number;
  /** NPC 对话 + Baker 消息链特殊信息 */
  special?: MissionSpecial;
}

/** 通过地图交互点位获得（method = 'map'） */
export interface MapAcquisition extends ArchiveAcquisitionContractBase {
  method: 'map';
  /** 地图节点 logicId */
  logicId: number;
}

/** 通过特殊交互获得（method = 'spec'，如浮空回收器） */
export interface SpecAcquisition extends ArchiveAcquisitionContractBase {
  method: 'spec';
  special: SpecSpecial;
}

/** 系统 / 百科自动解锁（method = 'auto'），无附加字段 */
export interface AutoAcquisition extends ArchiveAcquisitionContractBase {
  method: 'auto';
}

/** 通过商店兑换获得（method = 'shop'） */
export interface ShopAcquisition extends ArchiveAcquisitionContractBase {
  method: 'shop';
  /** 商店组 id（如 domainshop_map01） */
  shopGroupId: string;
  /** 商店页 id（如 domainshop_page_com_map01） */
  shopId: string;
  /** 售卖 NPC id */
  npcId?: string;
  /** 商店条目 logicId 列表 */
  logicIds?: number[];
}

/** 通过研究提交后解锁（method = 'invstgt'） */
export interface InvstgtAcquisition extends ArchiveAcquisitionContractBase {
  method: 'invstgt';
  /** 研究 id（如 research_landbreakerMurder） */
  researchId: string;
}

/** 单条档案获取契约（以 method 判别的联合类型） */
export type ArchiveAcquisitionContract =
  | MissionAcquisition
  | MapAcquisition
  | SpecAcquisition
  | AutoAcquisition
  | ShopAcquisition
  | InvstgtAcquisition;
