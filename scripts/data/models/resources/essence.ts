export type TermKind = 'attribute' | 'secondary' | 'skill';
export type WeaponType = 'sword' | 'claymore' | 'lance' | 'pistol' | 'wand';

export interface EssenceTerm {
  id: string;
  name: string;
  kind: TermKind;
  sortOrder: number;
  iconId: string | null;
}

export interface EssenceWeapon {
  id: string;
  name: string;
  rarity: number;
  type: WeaponType;
  iconId: string;
  attributeTermId: string | null;
  secondaryTermId: string | null;
  skillTermId: string | null;
}

export interface EssenceResource {
  version: number;
  terms: EssenceTerm[];
}

export interface WeaponsResource {
  version: number;
  weapons: EssenceWeapon[];
}
