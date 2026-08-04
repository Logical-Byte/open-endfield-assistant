/**
 * 生成档案库数据（resources/data/prts.json）。
 *
 * ## 输入（由 scripts/gameData.ts 读取的游戏原始数据表）
 * - `prtsPageTable`：页面定义（音像存档 / 见闻辑录 / 中枢档案）
 * - `prtsCategoryTable`：分类定义（藏品、电子档案、纸质记录……）
 * - `prtsFirstLvTable`：一级条目（分类 → 一级条目的从属）
 * - `prtsAllItemTable`：具体档案条目（一级条目 → 条目，含 name / type / contentId）
 * - `richContentTable`：富文本内容（用于解析文档 / 文本条目的标题）
 *
 * ## 输出
 * `PrtsData`（类型见 src/lib/prts.ts）：四个部分，均为「以 id 为键、
 * 写入顺序即展示顺序」的 Record，构成 页面 → 分类 → 一级条目 → 条目 的四级树。
 *
 * ## 关键逻辑
 * 1. 分类 → 页面没有显式字段，由该分类下 item 的 type 推断（取出现次数最多的）；
 * 2. 标题：音像存档与名称一致；文档 / 文本用 contentId 查富文本表，查不到回退为名称；
 * 3. 四个部分统一按 (页面、分类 order、一级条目 order、条目 order) 层级排序；
 * 4. 构建顺序遵循 页面 → 分类 → 一级条目 → 条目 的逻辑层级，具体条目放在最后；
 *    条目的标题解析 / 输出构建抽成独立函数（resolveItemTitle / buildAllItemOutputs），
 *    一级条目的单 item 名称一致性检查直接读取源表，不依赖条目中间产物。
 */
import type {
  PrtsAllItem,
  PrtsCategory,
  PrtsData,
  PrtsFirstLv,
  PrtsPage,
  PrtsPageType,
} from '../../src/lib/prts';
import type { PrtsAllItemEntry, PrtsFirstLvEntry } from '../models';
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

/**
 * 将有序的 [key, value] 列表转换为对象。
 *
 * 普通对象会保持键的写入顺序，因此调用方先排序、再转换，
 * 即可让最终 JSON 的字段顺序与展示顺序一致。
 */
function toRecord<T>(entries: [string, T][]): Record<string, T> {
  const result: Record<string, T> = {};
  for (const [key, value] of entries) {
    result[key] = value;
  }
  return result;
}

/**
 * 获取 pageType 在展示顺序中的排名。
 * 未收录在 PAGE_ORDER 中的类型排在最后（返回 PAGE_ORDER.length），
 * 保证排序时未知类型不会插队到已知页面之间。
 */
function pageRank(pageType: string): number {
  const index = PAGE_ORDER.indexOf(pageType as (typeof PAGE_ORDER)[number]);
  return index === -1 ? PAGE_ORDER.length : index;
}

/**
 * 计算每个 category 所属的 pageType。
 *
 * 数据表中没有显式的分类 - 页面关系，因此按该分类下 item 的 type 推断
 * （取出现次数最多的 type 作为所属页面）。
 *
 * 若某个分类下的 item 分布在多个页面（countByPage.size > 1），说明数据异常，
 * 打印警告以便人工核查；此时仍取出现次数最多的作为最终归属。
 */
function computeCategoryToPageType(): Record<string, string> {
  const categoryPage: Record<string, string> = {};
  // 遍历所有分类，统计每个分类下 item 在各类页面上的分布
  for (const categoryId of Object.keys(prtsCategoryTable)) {
    const countByPage = new Map<string, number>();
    // 先找该分类下所有一级条目，再遍历其 item
    for (const firstLvEntry of Object.values(prtsFirstLvTable)) {
      if (firstLvEntry.categoryId !== categoryId) continue;
      for (const itemId of firstLvEntry.itemIds) {
        const item = prtsAllItemTable[itemId];
        if (!item) continue;
        // item 的 type 即为所属的 pageType，累加计数
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
    // 取出现次数最多的页面作为该分类的归属（默认 text，计数为 0 时不改变默认值）
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

/**
 * 解析条目的展示标题。
 * 音像存档（multi_media）标题与名称一致；
 * 文档 / 文本以 contentId 在富文本表中查找，查不到时回退为名称。
 */
function resolveItemTitle(entry: PrtsAllItemEntry, name: string): string {
  if (entry.type === 'multi_media') {
    return name;
  }
  const content = richContentTable[entry.contentId];
  return content ? getTranslation(content.title, 'CN') : name;
}

/**
 * 单 item 一级条目的名称一致性检查。
 * 若一级条目名称与唯一 item 的名称或标题不一致，打印警告。
 * 独立解析 item 名称 / 标题，不依赖 allItemOutputs，可在一级条目之前调用。
 */
function warnIfSingleItemNameMismatch(entry: PrtsFirstLvEntry, name: string): void {
  if (entry.itemIds.length !== 1) {
    return;
  }
  const item = prtsAllItemTable[entry.itemIds[0]];
  if (!item) {
    return;
  }
  const itemName = getTranslation(item.name, 'CN');
  const itemTitle = resolveItemTitle(item, itemName);
  if (itemName !== name || itemTitle !== name) {
    console.warn(
      `[makePrts] ${entry.firstLvId} 的一级条目名称「${name}」与唯一 item「${item.id}」的名称「${itemName}」或标题「${itemTitle}」不一致`,
    );
  }
}

/** 生成 prts.json 的完整数据（PrtsData）。 */
export function makePrts(): PrtsData {
  // ===== 第一步：计算各层级的父子从属关系（分类 → 页面、页面 → 分类、分类 → 一级条目） =====
  const categoryToPageType = computeCategoryToPageType();

  // 页面 → 其下的分类 id 列表（基于上一步推断的归属，按分类 order 排序）
  const pageToCategoryIds: Record<string, string[]> = {};
  for (const [categoryId, page] of Object.entries(categoryToPageType)) {
    (pageToCategoryIds[page] ??= []).push(categoryId);
  }
  for (const page of Object.keys(pageToCategoryIds)) {
    pageToCategoryIds[page].sort((a, b) => prtsCategoryTable[a].order - prtsCategoryTable[b].order);
  }

  // 分类 → 其下的一级条目 id 列表（按一级条目 order 排序）
  const categoryToFirstLvIds: Record<string, string[]> = {};
  for (const entry of Object.values(prtsFirstLvTable)) {
    (categoryToFirstLvIds[entry.categoryId] ??= []).push(entry.firstLvId);
  }
  for (const categoryId of Object.keys(categoryToFirstLvIds)) {
    categoryToFirstLvIds[categoryId].sort(
      (a, b) => prtsFirstLvTable[a].order - prtsFirstLvTable[b].order,
    );
  }

  // ===== 第二步：页面部分（按 音像存档、见闻辑录、中枢档案 排序） =====
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

  // ===== 第三步：分类部分（先按所属页面排序，再按分类 order 排序） =====
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
        // 先比所属页面（页面本身有固定顺序），再比分类 order
        const pageDiff =
          pageRank(categoryToPageType[a[1].categoryId]) -
          pageRank(categoryToPageType[b[1].categoryId]);
        return pageDiff !== 0 ? pageDiff : a[1].order - b[1].order;
      }),
  );

  // ===== 第四步：一级条目部分（构建输出，含单 item 名称一致性检查） =====
  const firstLv = toRecord(
    Object.values(prtsFirstLvTable)
      .map((entry) => {
        const name = getTranslation(entry.name, 'CN');
        // 仅含单个 item 的一级条目：名称与唯一 item 的名称或标题不一致时打印警告
        warnIfSingleItemNameMismatch(entry, name);
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
      })
      // 一级条目：按 (页面、分类 order、一级条目 order) 排序
      .sort((a, b) => {
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

  // ===== 第五步：具体条目部分（构建输出后，按 页面、分类 order、一级条目 order、条目 order 排序） =====
  const allItemOutputs: Record<string, PrtsAllItem> = {};
  for (const entry of Object.values(prtsAllItemTable)) {
    const name = getTranslation(entry.name, 'CN');
    const title = resolveItemTitle(entry, name);
    // 标题与名称不一致时打印警告，便于核对源数据是否有误
    if (title !== name) {
      console.warn(`[makePrts] ${entry.id} 的标题「${title}」与名称「${name}」不一致`);
    }
    // categoryId 从所属一级条目推导（item 表本身不直接给出分类）
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

  const allItems = toRecord(
    Object.values(prtsAllItemTable)
      .map((entry) => [entry.id, allItemOutputs[entry.id]] as [string, PrtsAllItem])
      .sort((a, b) => {
        // 每个 item 的层级位置由所属一级条目决定，故先查其 firstLv 再逐级比较
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
