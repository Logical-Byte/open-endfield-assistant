import type { TranslationKey } from '../common';

export interface SkillPatchBlackboardEntry {
  key: string;
  value: number;
  valueStr: string;
}

export interface SkillPatchDataBundle {
  blackboard: SkillPatchBlackboardEntry[];
  coolDown: number;
  costType: number;
  costValue: number;
  description: TranslationKey;
  iconBgType: number;
  iconId: string;
  level: number;
  maxChargeTime: number;
  skillId: string;
  skillName: TranslationKey;
  subDescList: unknown[];
  subDescNameList: unknown[];
  tagId: string;
}

export interface SkillPatch {
  SkillPatchDataBundle: SkillPatchDataBundle[];
}

export type SkillPatchTable = Record<string, SkillPatch>;
