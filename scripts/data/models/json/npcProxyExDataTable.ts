/** NpcProxyExDataTable.json data 中单条记录的环境对话数据 */
export interface NpcProxyExEnvTalkData {
  envTalkIds: string[];
  envTalkOdd: number[];
  envTalkOverrideNpc: boolean;
}

/** NpcProxyExDataTable.json data[proxyId] 数组中的单条记录 */
export interface NpcProxyExData {
  addDialogExOption: boolean;
  envTalkData: NpcProxyExEnvTalkData;
  dialogExOptionData: unknown[];
  dialogId: string;
  missionId: string;
}

/** NpcProxyExDataTable.json proxyInfoData 中的单条代理信息 */
export interface NpcProxyInfo {
  npcProxyType: string;
  /** NPC id（如 a1m11daimeng_map01） */
  npcId: string;
  npcNameId: string;
  mapId: string;
}

/**
 * NpcProxyExDataTable.json。
 * 脚本只依赖 `proxyInfoData[proxyId].npcId`。
 */
export interface NpcProxyExDataTable {
  data: Record<string, NpcProxyExData[]>;
  proxyInfoData: Record<string, NpcProxyInfo>;
  proxyNumId2Str?: Record<string, unknown>;
  npcId2EnitiyData?: Record<string, unknown>;
}
