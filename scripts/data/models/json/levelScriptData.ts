/**
 * Json/LevelScriptData/<scene>/<scriptId>.json。
 *
 * 关卡脚本为任意嵌套的节点树。脚本通过遍历所有对象查找
 * `$type` 包含 ShowUIReadingPopPanel / CheckQuestState 的节点，
 * 并读取 `_readingPopId.constValue`、`_questId.constValue`。
 */

/** 脚本节点（可能包含 $type 与任意属性） */
export interface LevelScriptNode {
  $type?: string;
  [key: string]: unknown;
}

/** 脚本节点中的 constValue 容器（如 _questId / _readingPopId） */
export interface LevelScriptConstValue {
  constValue: number | string;
}

/** 关卡脚本顶层结构（不同脚本的顶层字段略有差异） */
export interface LevelScriptData {
  scriptId: number | string;
  allowTick: boolean;
  allowStartOnTravelPole: boolean;
  startType: string;
  endType: string;
  resetModeWhenActive: string;
  resetModeWhenEnd: string;
  activeShapeList: unknown[];
  actionMap: unknown;
  modules: Record<string, unknown>;
  enemies: Record<string, unknown>;
  interactives: Record<string, unknown>;
  npcs: Record<string, unknown>;
  interactiveLocks: Record<string, unknown>;
  triggerVolumes: Record<string, unknown>;
}
