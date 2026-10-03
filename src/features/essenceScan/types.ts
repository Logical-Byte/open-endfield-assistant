export type StatKind = 'attribute' | 'secondary' | 'skill';
export interface EssenceStat {
  id: string;
  name: string;
  kind: StatKind;
}
export interface EssenceWeapon {
  id: string;
  name: string;
  rarity: number;
  stats: [string | null, string | null, string | null];
}
export interface EssenceCatalog {
  stats: EssenceStat[];
  weapons: EssenceWeapon[];
}

/** 一次扫描使用后端已保存设置的快照。三个阈值按属性类型排列。 */
export interface EssenceScanSettings {
  nonFiveStar: 'process' | 'skip';
  protectLocked: boolean;
  skipAbandoned: boolean;
  highLevel: readonly [number, number, number] | null;
  excludedWeaponIds: readonly string[];
  customKeeps: readonly (readonly [string, string, string])[];
}

export interface Essence {
  stats: [string | null, string | null, string | null];
  levels: [number | null, number | null, number | null];
  rarity: 'five' | 'four' | 'other' | 'unknown';
  locked: boolean | null;
  abandoned: boolean | null;
}

export type Decision = 'keep' | 'discard' | 'skip' | 'review';
export interface Evaluation {
  decision: Decision;
  reason:
    | 'locked'
    | 'abandoned'
    | 'nonFiveStar'
    | 'customRule'
    | 'highLevel'
    | 'weaponMatch'
    | 'excludedWeapons'
    | 'noMatchingWeapon'
    | 'incompleteRecognition';
  matchedWeaponIds: string[];
}
