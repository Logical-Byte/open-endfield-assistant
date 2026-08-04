/**
 * 档案库数据（resources/data/prts.json）的类型定义。
 *
 * ## 数据结构
 *
 * prts.json 由四部分（PrtsPage / PrtsCategory / firstLv / allItems）组成，
 * 它们构成一棵四级树形结构，从页面逐级下钻到具体档案条目：
 *
 *   PrtsPage（页面，键 = pageType）
 *     └─ categoryIds → PrtsCategory（分类，键 = categoryId）
 *          └─ firstLvIds → PrtsFirstLv（一级条目，键 = firstLvId）
 *               └─ itemIds → PrtsAllItem（具体条目，键 = id）
 *
 * 各部分的键均为对应实体的 id（Record 键），值中通过 id 数组（categoryIds /
 * firstLvIds / itemIds）表达父子关系；同时子实体（category / firstLv / item）
 * 反向持有父级 id（categoryId / firstLvId），便于向上回溯。
 *
 * ## 数据来源
 *
 * 生成逻辑见 `scripts/tasks/makePrts.ts`：
 * - 页面→分类、分类→一级条目的归属关系来自游戏数据表；
 * - 分类→页面没有显式字段，由该分类下 item 的 type 推断（取出现次数最多的）；
 * - 四个部分的 Record 写入顺序即前端展示顺序，统一按
 *   (页面、分类 order、一级条目 order、条目 order) 层级排序。
 */

/** 档案库页面类型（音像存档 / 见闻辑录 / 中枢档案） */
export type PrtsPageType = 'multi_media' | 'text' | 'document';

/**
 * 档案库页面（音像存档 / 见闻辑录 / 中枢档案）。
 * PrtsData.PrtsPage 以 pageType 为键。
 */
export interface PrtsPage {
  /** 页面名称（中文，取自 i18n 文本表） */
  name: string;
  /** 页面类型，同时作为本页面的唯一标识（Record 键） */
  pageType: PrtsPageType;
  /** 该页面下的分类 id 列表（按所属页面、分类 order 排序） */
  categoryIds: string[];
}

/**
 * 档案库分类（藏品、电子档案、纸质记录……）。
 * PrtsData.PrtsCategory 以 categoryId 为键。
 */
export interface PrtsCategory {
  /** 分类唯一标识（Record 键） */
  categoryId: string;
  /** 分类名称（中文） */
  name: string;
  /** 分类在所属页面内的展示顺序（数字越小越靠前） */
  order: number;
  /**
   * 所属页面类型。
   * 数据表无显式关系，由该分类下 item 的 type 推断（取多数），见 makePrts.ts。
   */
  type: PrtsPageType;
  /** 该分类下的一级条目 id 列表（按一级条目 order 排序） */
  firstLvIds: string[];
}

/**
 * 一级条目（页面 → 分类下的中间层级）。
 * PrtsData.firstLv 以 firstLvId 为键。
 */
export interface PrtsFirstLv {
  /** 所属分类 id（反向引用 PrtsCategory.categoryId） */
  categoryId: string;
  /** 一级条目唯一标识（Record 键） */
  firstLvId: string;
  /** 该一级条目下的具体档案条目 id 列表（保持数据表原始顺序） */
  itemIds: string[];
  /** 一级条目名称（中文） */
  name: string;
  /** 一级条目在所属分类内的展示顺序（数字越小越靠前） */
  order: number;
  /** 所属页面类型（与所属分类的 type 保持一致） */
  type: PrtsPageType;
}

/**
 * 具体档案条目（树形结构的叶子节点）。
 * PrtsData.allItems 以 id 为键。
 */
export interface PrtsAllItem {
  /** 所属分类 id（由所属一级条目的 categoryId 推导） */
  categoryId: string;
  /** 所属一级条目 id（反向引用 PrtsFirstLv.firstLvId） */
  firstLvId: string;
  /** 档案条目唯一标识（Record 键） */
  id: string;
  /** 条目名称（中文） */
  name: string;
  /** 条目在所属一级条目内的展示顺序（数字越小越靠前） */
  order: number;
  /**
   * 展示标题（中文）：音像存档（multi_media）与名称一致；
   * 文档 / 文本则以 contentId 在富文本表中查找，查不到时回退为名称。
   */
  title: string;
  /** 条目所属页面类型 */
  type: PrtsPageType;
}

/**
 * prts.json 完整结构。
 * 四个部分均为「以 id 为键的 Record」，值中通过 id 数组表达层级关系；
 * Record 的键写入顺序即前端展示顺序（见 makePrts.ts 的排序逻辑）。
 */
export interface PrtsData {
  /** 档案库页面（键 = pageType，按 音像存档 → 见闻辑录 → 中枢档案 排序） */
  PrtsPage: Record<string, PrtsPage>;
  /** 档案库分类（键 = categoryId，按 页面、分类 order 排序） */
  PrtsCategory: Record<string, PrtsCategory>;
  /** 一级条目（键 = firstLvId，按 页面、分类 order、一级条目 order 排序） */
  firstLv: Record<string, PrtsFirstLv>;
  /** 具体档案条目（键 = id，按 页面、分类 order、一级条目 order、条目 order 排序） */
  allItems: Record<string, PrtsAllItem>;
}
