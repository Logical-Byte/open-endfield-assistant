/**
 * 档案库数据（resources/data/prts.json）的类型定义。
 */

/** 档案库页面类型（音像存档 / 见闻辑录 / 中枢档案） */
export type PrtsPageType = 'multi_media' | 'text' | 'document';

/** 档案库页面（音像存档 / 见闻辑录 / 中枢档案） */
export interface PrtsPage {
  /** 页面名称 */
  name: string;
  /** 页面类型 */
  pageType: PrtsPageType;
  /** 该页面下的分类 id（按所属页面、order 排序） */
  categoryIds: string[];
}

/** 档案库分类（藏品、电子档案、纸质记录……） */
export interface PrtsCategory {
  categoryId: string;
  name: string;
  order: number;
  /** 所属页面（与具体条目的 type 保持一致） */
  type: PrtsPageType;
  /** 该分类下的一级条目 id（按 order 排序） */
  firstLvIds: string[];
}

/** 一级条目 */
export interface PrtsFirstLv {
  categoryId: string;
  firstLvId: string;
  /** 关联的具体条目 id */
  itemIds: string[];
  name: string;
  order: number;
  /** 所属页面（与具体条目的 type 保持一致） */
  type: PrtsPageType;
}

/** 具体档案条目 */
export interface PrtsAllItem {
  /** 所属分类 id */
  categoryId: string;
  firstLvId: string;
  id: string;
  name: string;
  order: number;
  title: string;
  type: PrtsPageType;
}

/** prts.json 完整结构 */
export interface PrtsData {
  PrtsPage: Record<string, PrtsPage>;
  PrtsCategory: Record<string, PrtsCategory>;
  firstLv: Record<string, PrtsFirstLv>;
  allItems: Record<string, PrtsAllItem>;
}
