export interface LevelLoading {
  bgNameGroup: string[];
  levelId: string;
  mapTags: number[];
  originOverrideTypeTag: boolean;
  overrideTypeTags: number[];
  regionRelated: boolean;
  regularBgNameGroup: string[];
  regularTipsKeyGroup: string[];
  typeTags: number[];
}

export type LevelLoadingTable = Record<string, LevelLoading>;
