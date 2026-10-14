import fs from 'node:fs';
import path from 'node:path';
import { parseJSONWithBigInt } from '../readGameData';
import type { TranslationKey } from '../models/common';
import type { GemTable } from '../models/tableCfg/gemTable';
import type { GemTagIdTable } from '../models/tableCfg/gemTagIdTable';
import type { ItemTable } from '../models/tableCfg/itemTable';
import type { SkillPatchTable } from '../models/tableCfg/skillPatchTable';
import type { WeaponBasicTable } from '../models/tableCfg/weaponBasicTable';
import type {
  EssenceResource,
  TermKind,
  WeaponsResource,
  WeaponType,
} from '../models/resources/essence';

interface EssenceTables {
  gems: GemTable;
  tags: GemTagIdTable;
  items: ItemTable;
  skills: SkillPatchTable;
  weapons: WeaponBasicTable;
  texts: Record<string, string>;
}

export function readEssenceTables(dataDir: string): EssenceTables {
  function read<T>(name: string): T {
    return parseJSONWithBigInt(
      fs.readFileSync(path.join(dataDir, 'TableCfg', `${name}.json`), 'utf8'),
    );
  }

  return {
    gems: read<GemTable>('GemTable'),
    tags: read<GemTagIdTable>('GemTagIdTable'),
    items: read<ItemTable>('ItemTable'),
    skills: read<SkillPatchTable>('SkillPatchTable'),
    weapons: read<WeaponBasicTable>('WeaponBasicTable'),
    texts: read<Record<string, string>>('I18nTextTable_CN'),
  };
}

/** 原表关联只在生成阶段执行，完整性由 Rust 资源检查负责。 */
export function makeEssence({ gems, tags, items, skills, weapons, texts }: EssenceTables): {
  essence: EssenceResource;
  weapons: WeaponsResource;
} {
  // 文本 ID 可能超过 JS 安全整数范围，保留原始十进制字符串。
  function text(key: TranslationKey): string {
    return texts[String(key.id)] ?? key.text;
  }

  const termKinds: Record<number, TermKind> = {
    0: 'attribute',
    1: 'secondary',
    2: 'skill',
  };
  const weaponTypes: Record<number, WeaponType> = {
    1: 'sword',
    2: 'wand',
    3: 'claymore',
    5: 'lance',
    6: 'pistol',
  };

  return {
    essence: {
      version: 1,
      terms: Object.entries(gems)
        .map(([id, gem]) => ({
          id,
          name: text(gem.tagName),
          kind: termKinds[gem.termType],
          sortOrder: gem.sortOrder,
          iconId: gem.tagIcon || null,
        }))
        .sort((a, b) => a.sortOrder - b.sortOrder),
    },
    weapons: {
      version: 1,
      weapons: Object.entries(weapons).map(([id, weapon]) => {
        const item = items[id];
        const termIds = weapon.weaponSkillList.map((skillId) => {
          const tagId = skills[skillId]?.SkillPatchDataBundle[0]?.tagId;
          return tags[tagId];
        });
        function term(kind: number): string | null {
          return termIds.find((id) => gems[id]?.termType === kind) ?? null;
        }
        return {
          id,
          name: text(item.name),
          rarity: weapon.rarity,
          type: weaponTypes[weapon.weaponType],
          iconId: item.iconId,
          attributeTermId: term(0),
          secondaryTermId: term(1),
          skillTermId: term(2),
        };
      }),
    },
  };
}

export function writeEssence(tables: EssenceTables): void {
  const { essence, weapons } = makeEssence(tables);
  fs.writeFileSync('resources/data/essence.json', `${JSON.stringify(essence, null, 2)}\n`);
  fs.writeFileSync('resources/data/weapons.json', `${JSON.stringify(weapons, null, 2)}\n`);
}
