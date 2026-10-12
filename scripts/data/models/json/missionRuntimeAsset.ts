/** MissionRuntimeAsset 中 questDic 的单条子任务 */
export interface MissionQuest {
  questId: string;
  questType: number;
  objectiveConditionNum: number;
  rewardId: string;
  overrideMissionDesc: string;
  descriptionOverride: unknown;
  showMode: number;
  forceShowHudAnim: boolean;
  ignoreNewQuestAnim: boolean;
  ignoreQuestCompleteAnim: boolean;
  blockQuestSkipToast: boolean;
  needHudUpdateTag: boolean;
  objectiveList: unknown[];
  needItemIds: unknown[];
  prevQuestIdList: unknown[];
  flowIndex: number;
}

/**
 * MissionRuntimeAsset/<missionId>.json。
 * 脚本遍历 questDic 并在 JSON 文本中检索 scriptId。
 */
export interface MissionRuntimeAsset {
  missionId: string;
  missionName: unknown;
  rewardId: string;
  missionType: number;
  baseMissionImportance: number;
  overrideImportance: number;
  sortId: number;
  charId: string;
  missionDescription: unknown;
  levelId: string;
  scope: number;
  missionChapterBitmask: number;
  skipMissionAcceptAnim: boolean;
  skipMissionCompleteAnim: boolean;
  questDic: Record<string, MissionQuest>;
  actionMapRaw: unknown;
  clientActionMapKey: unknown;
  clientActionMapValue: unknown;
  onMissionAcceptId: unknown;
  onMissionCompletedId: unknown;
  onMissionFailedId: unknown;
  properties: unknown;
  propertyIdToKeyMap: unknown;
  propertyKeyToIdMap: unknown;
  isWrapperMission: boolean;
  useRewardWrapper: boolean;
  useLevelIdWrapper: boolean;
  externalInfo: unknown;
}
