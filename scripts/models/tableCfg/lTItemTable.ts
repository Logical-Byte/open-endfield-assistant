/** 单个限时道具条目 */
export interface LTItem {
  itemId: string;
}

/** LTItemTable.json 对应整个表 */
export type LTItemTable = Record<string, LTItem>;
