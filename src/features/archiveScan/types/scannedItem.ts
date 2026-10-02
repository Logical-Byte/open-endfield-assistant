export type ScannedItemStatus = 'success' | 'unrecognized' | 'failed';

/** 单份档案的扫描结果（与 Rust 侧 `ScannedItem` 对齐）。 */
export interface ScannedItem {
  /** 识别状态：success（纠错成功）| unrecognized（识别到文本但无法纠错）| failed（OCR 为空） */
  status: ScannedItemStatus;
  /** 扫描时所在的档案库大类 id（pageType：multi_media / text / document） */
  foundInCategory: string;
  /** 扫描时所在的档案库小类 id（categoryId），限定人工纠错的匹配范围 */
  foundInSubCategory: string;
  /** 档案详情页面截图（base64 PNG data URL） */
  image: string;
  /** 原始 OCR 识别结果（人工纠错时保留） */
  ocrResult: string;
  /** 后端纠错或人工输入的标题（尚无纠错标题时为 null） */
  correctedTitle: string | null;
  /** 纠错命中的档案 id（allItems 的 id，当前小分类下同标题多条时返回全部） */
  correctedMatchItemIds: readonly string[];
}

export enum CollectType {
  Collected,
  Unrecognized,
  Failed,
  NotCollected,
}

export interface ScannedItemCardProps {
  collectType: CollectType;
  category: string;
  subCategory: string;
  imageUrl: string | null;
  title: string;
  archiveId: string | null;
  /** 对应的扫描结果对象（未收集卡片为 null，不可纠错） */
  scannedItem: ScannedItem | null;
}
