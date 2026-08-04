/** 单个恢复 AP 道具 */
export interface RecoverApItem {
  apRecoverValue: number;
  id: string;
}

/** RecoverApItemTable.json 对应整个表 */
export type RecoverApItemTable = Record<string, RecoverApItem>;
