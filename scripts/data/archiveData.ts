/** 档案生成只读取所需表及 CN/EN，不依赖其他语言或业务表。 */
import type {
  I18nTextTable,
  PrtsAllItem,
  PrtsCategory,
  PrtsFirstLv,
  PrtsPage,
  RichContentTable,
  TranslationKey,
} from './models';
import type { LocalizedText } from './models/resources/prts';
import { readJSONWithBigInt } from './readGameData';

export const prtsAllItemTable = readJSONWithBigInt<PrtsAllItem>('TableCfg/PrtsAllItem.json');
export const prtsCategoryTable = readJSONWithBigInt<PrtsCategory>('TableCfg/PrtsCategory.json');
export const prtsFirstLvTable = readJSONWithBigInt<PrtsFirstLv>('TableCfg/PrtsFirstLv.json');
export const prtsPageTable = readJSONWithBigInt<PrtsPage>('TableCfg/PrtsPage.json');
export const richContentTable = readJSONWithBigInt<RichContentTable>(
  'TableCfg/RichContentTable.json',
);
type ResourceLocale = keyof LocalizedText;

function loadTextTable(locale: ResourceLocale, language: 'CN' | 'EN'): I18nTextTable {
  try {
    return readJSONWithBigInt<I18nTextTable>(`TableCfg/I18nTextTable_${language}.json`);
  } catch {
    throw new Error(`[makePrts] ${locale} 源文本表缺失或无效`);
  }
}
const tables: Record<ResourceLocale, I18nTextTable> = {
  'zh-CN': loadTextTable('zh-CN', 'CN'),
  'en-US': loadTextTable('en-US', 'EN'),
};

/** 每个必需字段严格取同一键的两种译文，源 text 不承担回退。 */
export function getArchiveText(key: TranslationKey, entity: string, field: string): LocalizedText {
  function translation(locale: ResourceLocale): string {
    const id = String(key.id);
    const location = `${locale} ${entity}.${field} TranslationKey=${id}`;
    if (id === '0') throw new Error(`[makePrts] ${location} 零 ID 空引用`);
    const text = tables[locale][id];
    if (text === undefined) throw new Error(`[makePrts] ${location} 非零键缺译`);
    if (!text.trim()) throw new Error(`[makePrts] ${location} 译文为空白`);
    return text.trim();
  }
  return { 'zh-CN': translation('zh-CN'), 'en-US': translation('en-US') };
}
