import * as fs from 'node:fs';
import path from 'node:path';
import type {
  BattlePassLevelTable,
  BattlePassOverrideLevelTable,
  BattlePassSeasonTable,
  BattlePassTrackTable,
  CashShopGoodsTable,
  CashShopGroupTable,
  CashShopHideInGameTable,
  CashShopTable,
  CharWpnRecommendTable,
  GemTable,
  GemTagIdTable,
  GiftpackCashShopGoodsDataTable,
  I18nTextTable,
  ItemListByTypeTable,
  ItemTable,
  ItemTypeTable,
  LevelLoadingTable,
  LTItemTable,
  PrtsAllItem,
  PrtsCategory,
  PrtsFirstLv,
  PrtsPage,
  RecoverApItemTable,
  RewardTable,
  RichContentTable,
  SimulationTrainingCardPoolTable,
  SimulationTrainingCardTable,
  SimulationTrainingConst,
  SimulationTrainingLevelTable,
  SkillPatchTable,
  TextTable,
  TranslationKey,
  WeaponBasicTable,
  WorldEnergyPointGroupTable,
  WorldEnergyPointTable,
} from './models';

export { endfieldDataDir, parseJSONWithBigInt, readJSONWithBigInt } from './readGameData';
import { endfieldDataDir, readJSONWithBigInt } from './readGameData';

export type I18nLanguage = (typeof i18nLanguages)[number];
export type Locale = (typeof languageToLocaleMap)[I18nLanguage];

/** 获取指定语言的国际化文本表路径 */
export function getI18nTextTablePath(language: I18nLanguage): string {
  const I18nDir = path.join(endfieldDataDir, 'TableCfg');
  return path.join(I18nDir, `I18nTextTable_${language}.json`);
}

/**
 * 获取指定语言的文本内容
 * 如果找不到翻译或翻译为空，返回原始文本
 */
export function getTranslation({ id, text }: TranslationKey, language: I18nLanguage): string {
  const translation = i18nTextTables.get(language)?.[String(id)];
  if (translation !== undefined) {
    return translation.trim();
  } else {
    return text;
  }
}

export const i18nLanguages = [
  'CN',
  'TC',
  'DE',
  'EN',
  'MX',
  'FR',
  'ID',
  'IT',
  'JP',
  'KR',
  'BR',
  'RU',
  'TH',
  'VN',
] as const;
export const languageToLocaleMap = {
  CN: 'zh-CN',
  TC: 'zh-TW',
  DE: 'de-DE',
  EN: 'en-US',
  MX: 'es-MX',
  FR: 'fr-FR',
  ID: 'id-ID',
  IT: 'it-IT',
  JP: 'ja-JP',
  KR: 'ko-KR',
  BR: 'pt-BR',
  RU: 'ru-RU',
  TH: 'th-TH',
  VN: 'vi-VN',
} as const;

// 读取文件
export const battlePassLevelTable: BattlePassLevelTable = readJSONWithBigInt(
  'TableCfg/BattlePassLevelTable.json',
);
export const battlePassOverrideLevelTable: BattlePassOverrideLevelTable = readJSONWithBigInt(
  'TableCfg/BattlePassOverrideLevelTable.json',
);
export const battlePassSeasonTable: BattlePassSeasonTable = readJSONWithBigInt(
  'TableCfg/BattlePassSeasonTable.json',
);
export const battlePassTrackTable: BattlePassTrackTable = readJSONWithBigInt(
  'TableCfg/BattlePassTrackTable.json',
);
export const cashShopGoodsTable: CashShopGoodsTable = readJSONWithBigInt(
  'TableCfg/CashShopGoodsTable.json',
);
export const cashShopGroupTable: CashShopGroupTable = readJSONWithBigInt(
  'TableCfg/CashShopGroupTable.json',
);
export const cashShopHideInGameTable: CashShopHideInGameTable = readJSONWithBigInt(
  'TableCfg/CashShopHideInGameTable.json',
);
export const cashShopTable: CashShopTable = readJSONWithBigInt('TableCfg/CashShopTable.json');
export const charWpnRecommendTable: CharWpnRecommendTable = readJSONWithBigInt(
  'TableCfg/CharWpnRecommendTable.json',
);
export const gemTable: GemTable = readJSONWithBigInt('TableCfg/GemTable.json');
export const gemTagIdTable: GemTagIdTable = readJSONWithBigInt('TableCfg/GemTagIdTable.json');
export const giftpackCashShopGoodsDataTable: GiftpackCashShopGoodsDataTable = readJSONWithBigInt(
  'TableCfg/GiftpackCashShopGoodsDataTable.json',
);
export const itemListByTypeTable: ItemListByTypeTable = readJSONWithBigInt(
  'TableCfg/ItemListByTypeTable.json',
);
export const itemTable: ItemTable = readJSONWithBigInt('TableCfg/ItemTable.json');
export const itemTypeTable: ItemTypeTable = readJSONWithBigInt('TableCfg/ItemTypeTable.json');
export const levelLoadingTable: LevelLoadingTable = readJSONWithBigInt(
  'TableCfg/LevelLoadingTable.json',
);
export const lTItemTable: LTItemTable = readJSONWithBigInt('TableCfg/LTItemTable.json');
export const prtsAllItemTable: PrtsAllItem = readJSONWithBigInt('TableCfg/PrtsAllItem.json');
export const prtsCategoryTable: PrtsCategory = readJSONWithBigInt('TableCfg/PrtsCategory.json');
export const prtsFirstLvTable: PrtsFirstLv = readJSONWithBigInt('TableCfg/PrtsFirstLv.json');
export const prtsPageTable: PrtsPage = readJSONWithBigInt('TableCfg/PrtsPage.json');
export const recoverApItemTable: RecoverApItemTable = readJSONWithBigInt(
  'TableCfg/RecoverApItemTable.json',
);
export const rewardTable: RewardTable = readJSONWithBigInt('TableCfg/RewardTable.json');
export const richContentTable: RichContentTable = readJSONWithBigInt(
  'TableCfg/RichContentTable.json',
);
export const simulationTrainingCardPoolTable: SimulationTrainingCardPoolTable = readJSONWithBigInt(
  'TableCfg/SimulationTrainingCardPoolTable.json',
);
export const simulationTrainingCardTable: SimulationTrainingCardTable = readJSONWithBigInt(
  'TableCfg/SimulationTrainingCardTable.json',
);
export const simulationTrainingConst: SimulationTrainingConst = readJSONWithBigInt(
  'TableCfg/SimulationTrainingConst.json',
);
export const simulationTrainingLevelTable: SimulationTrainingLevelTable = readJSONWithBigInt(
  'TableCfg/SimulationTrainingLevelTable.json',
);
export const skillPatchTable: SkillPatchTable = readJSONWithBigInt('TableCfg/SkillPatchTable.json');
export const textTable: TextTable = readJSONWithBigInt('TableCfg/TextTable.json');
export const weaponBasicTable: WeaponBasicTable = readJSONWithBigInt(
  'TableCfg/WeaponBasicTable.json',
);
export const worldEnergyPointGroupTable: WorldEnergyPointGroupTable = readJSONWithBigInt(
  'TableCfg/WorldEnergyPointGroupTable.json',
);
export const worldEnergyPointTable: WorldEnergyPointTable = readJSONWithBigInt(
  'TableCfg/WorldEnergyPointTable.json',
);

export const i18nTextTables: Map<string, I18nTextTable> = new Map(
  i18nLanguages.map((lang) => [
    lang,
    JSON.parse(fs.readFileSync(getI18nTextTablePath(lang), 'utf8')),
  ]),
);
