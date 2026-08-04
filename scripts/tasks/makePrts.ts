import type {
  PrtsAllItem,
  PrtsCategory,
  PrtsData,
  PrtsFirstLv,
  PrtsPage,
  PrtsPageType,
} from '../../src/lib/prts';
import {
  getTranslation,
  prtsAllItemTable,
  prtsCategoryTable,
  prtsFirstLvTable,
  prtsPageTable,
  richContentTable,
} from '../gameData';

/** 档案库页面展示顺序：音像存档、见闻辑录、中枢档案 */
const PAGE_ORDER = ['multi_media', 'text', 'document'] as const;

/** 获取 pageType 在展示顺序中的排名（未收录的排在最后） */
function pageRank(pageType: string): number {
  const index = PAGE_ORDER.indexOf(pageType as (typeof PAGE_ORDER)[number]);
  return index === -1 ? PAGE_ORDER.length : index;
}

/**
 * 计算每个 category 所属的 pageType。
 * 数据表中没有显式的分类 - 页面关系，按该分类下 item 的 type 推断（取出现次数最多的）。
 */
function computeCategoryToPageType(): Record<string, string> {
  const categoryPage: Record<string, string> = {};
  for (const categoryId of Object.keys(prtsCategoryTable)) {
    const countByPage = new Map<string, number>();
    for (const firstLvEntry of Object.values(prtsFirstLvTable)) {
      if (firstLvEntry.categoryId !== categoryId) continue;
      for (const itemId of firstLvEntry.itemIds) {
        const item = prtsAllItemTable[itemId];
        if (!item) continue;
        // item 的 type 即为所属的 pageType
        countByPage.set(item.type, (countByPage.get(item.type) ?? 0) + 1);
      }
    }
    // 期望每个分类唯一位于一个 page；若 item 分布在多个 page，则给出警告
    if (countByPage.size > 1) {
      const distribution = [...countByPage.entries()]
        .map(([page, count]) => `${page}=${count}`)
        .join(', ');
      console.warn(
        `[makePrts] category ${categoryId} 的 item 分布在多个 page（${distribution}），无法唯一定位所属页面`,
      );
    }
    let bestPage = 'text';
    let bestCount = 0;
    for (const [page, count] of countByPage) {
      if (count > bestCount) {
        bestPage = page;
        bestCount = count;
      }
    }
    categoryPage[categoryId] = bestPage;
  }
  return categoryPage;
}

/** 将有序的 [key, value] 列表转换为对象（保持键的写入顺序） */
function toRecord<T>(entries: [string, T][]): Record<string, T> {
  const result: Record<string, T> = {};
  for (const [key, value] of entries) {
    result[key] = value;
  }
  return result;
}

export function makePrts(): PrtsData {
  // 分类 → 页面，以及页面 → 分类、分类 → 一级条目的从属关系（按各自 order 排序）
  const categoryToPageType = computeCategoryToPageType();
  const categoryToFirstLvIds: Record<string, string[]> = {};
  for (const entry of Object.values(prtsFirstLvTable)) {
    (categoryToFirstLvIds[entry.categoryId] ??= []).push(entry.firstLvId);
  }
  for (const categoryId of Object.keys(categoryToFirstLvIds)) {
    categoryToFirstLvIds[categoryId].sort(
      (a, b) => prtsFirstLvTable[a].order - prtsFirstLvTable[b].order,
    );
  }
  const pageToCategoryIds: Record<string, string[]> = {};
  for (const [categoryId, page] of Object.entries(categoryToPageType)) {
    (pageToCategoryIds[page] ??= []).push(categoryId);
  }
  for (const page of Object.keys(pageToCategoryIds)) {
    pageToCategoryIds[page].sort((a, b) => prtsCategoryTable[a].order - prtsCategoryTable[b].order);
  }

  // 具体条目：构建输出（含标题检查）
  const allItemOutputs: Record<string, PrtsAllItem> = {};
  for (const entry of Object.values(prtsAllItemTable)) {
    const name = getTranslation(entry.name, 'CN');
    // 多媒体：标题与名称一致；文档 / 文本：以 contentId 在 RichContentTable 中查找标题
    let title: string;
    if (entry.type === 'multi_media') {
      title = name;
    } else {
      const content = richContentTable[entry.contentId];
      title = content ? getTranslation(content.title, 'CN') : name;
    }
    if (title !== name) {
      console.warn(`[makePrts] ${entry.id} 的标题「${title}」与名称「${name}」不一致`);
    }
    allItemOutputs[entry.id] = {
      categoryId: prtsFirstLvTable[entry.firstLvId].categoryId,
      firstLvId: entry.firstLvId,
      id: entry.id,
      name,
      order: entry.order,
      title,
      type: entry.type as PrtsPageType,
    };
  }

  // 页面：按 音像存档、见闻辑录、中枢档案 排序
  const PrtsPage = toRecord(
    Object.values(prtsPageTable)
      .map(
        (entry) =>
          [
            entry.pageType,
            {
              name: getTranslation(entry.name, 'CN'),
              pageType: entry.pageType as PrtsPageType,
              categoryIds: pageToCategoryIds[entry.pageType] ?? [],
            },
          ] as [string, PrtsPage],
      )
      .sort((a, b) => pageRank(a[1].pageType) - pageRank(b[1].pageType)),
  );

  // 分类：先按所属 page 排序，再按 order 排序
  const PrtsCategory = toRecord(
    Object.values(prtsCategoryTable)
      .map(
        (entry) =>
          [
            entry.categoryId,
            {
              categoryId: entry.categoryId,
              name: getTranslation(entry.name, 'CN'),
              order: entry.order,
              type: categoryToPageType[entry.categoryId] as PrtsPageType,
              firstLvIds: categoryToFirstLvIds[entry.categoryId] ?? [],
            },
          ] as [string, PrtsCategory],
      )
      .sort((a, b) => {
        const pageDiff =
          pageRank(categoryToPageType[a[1].categoryId]) -
          pageRank(categoryToPageType[b[1].categoryId]);
        return pageDiff !== 0 ? pageDiff : a[1].order - b[1].order;
      }),
  );

  // 一级条目：构建输出（含单 item 名称一致性检查）
  const firstLvEntries: [string, PrtsFirstLv][] = Object.values(prtsFirstLvTable).map((entry) => {
    const name = getTranslation(entry.name, 'CN');
    // 仅含单个 item 的一级条目：若名称与唯一的 item 的名称、标题均不一致，打印警告
    if (entry.itemIds.length === 1) {
      const item = allItemOutputs[entry.itemIds[0]];
      if (item && item.name !== name && item.title !== name) {
        console.warn(
          `[makePrts] ${entry.firstLvId} 的一级条目名称「${name}」与唯一 item「${item.id}」的名称「${item.name}」、标题「${item.title}」均不一致`,
        );
      }
    }
    return [
      entry.firstLvId,
      {
        categoryId: entry.categoryId,
        firstLvId: entry.firstLvId,
        itemIds: entry.itemIds,
        name,
        order: entry.order,
        type: categoryToPageType[entry.categoryId] as PrtsPageType,
      },
    ] as [string, PrtsFirstLv];
  });

  // 一级条目：按 (page, category order, firstLv order) 排序
  const firstLv = toRecord(
    firstLvEntries.sort((a, b) => {
      const pageDiff =
        pageRank(categoryToPageType[a[1].categoryId]) -
        pageRank(categoryToPageType[b[1].categoryId]);
      if (pageDiff !== 0) return pageDiff;
      const categoryDiff =
        prtsCategoryTable[a[1].categoryId].order - prtsCategoryTable[b[1].categoryId].order;
      if (categoryDiff !== 0) return categoryDiff;
      return a[1].order - b[1].order;
    }),
  );

  // 具体条目：按 (page, category order, firstLv order, item order) 排序
  const allItems = toRecord(
    Object.values(prtsAllItemTable)
      .map((entry) => [entry.id, allItemOutputs[entry.id]] as [string, PrtsAllItem])
      .sort((a, b) => {
        const firstLvA = prtsFirstLvTable[a[1].firstLvId];
        const firstLvB = prtsFirstLvTable[b[1].firstLvId];
        const pageDiff =
          pageRank(categoryToPageType[firstLvA.categoryId]) -
          pageRank(categoryToPageType[firstLvB.categoryId]);
        if (pageDiff !== 0) return pageDiff;
        const categoryDiff =
          prtsCategoryTable[firstLvA.categoryId].order -
          prtsCategoryTable[firstLvB.categoryId].order;
        if (categoryDiff !== 0) return categoryDiff;
        if (firstLvA.order !== firstLvB.order) return firstLvA.order - firstLvB.order;
        return a[1].order - b[1].order;
      }),
  );

  return { PrtsPage, PrtsCategory, firstLv, allItems };
}
