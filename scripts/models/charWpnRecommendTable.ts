export interface CharWpnRecommend {
  charId: string;
  weaponIds1: string[];
  weaponIds2: string[];
  weaponIds3: string[];
}

export type CharWpnRecommendTable = Record<string, CharWpnRecommend>;
