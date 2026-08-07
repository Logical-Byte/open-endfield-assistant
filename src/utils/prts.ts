import { prtsData } from '@/utils/app/prtsData';

/** 大类 id（pageType）→ 中文名；找不到时回退为原 id。 */
export function getPageName(pageType: string): string {
  return prtsData.value?.PrtsPage[pageType]?.name ?? pageType;
}

/** 小类 id（categoryId）→ 中文名；找不到时回退为原 id。 */
export function getCategoryName(categoryId: string): string {
  return prtsData.value?.PrtsCategory[categoryId]?.name ?? categoryId;
}

/** 档案 id → 标题；找不到时回退为原 id。 */
export function getItemTitleById(id: string): string {
  return prtsData.value?.allItems[id]?.title ?? id;
}

/** 某子分类下所有档案标题 */
export function getCategoryTitles(categoryId: string): string[] {
  return Object.values(prtsData.value?.allItems ?? {})
    .filter((item) => item.categoryId === categoryId)
    .map((item) => item.title);
}
