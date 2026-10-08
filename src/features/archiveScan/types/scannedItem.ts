/** 档案目录 ID 与扫描记录 ID 在类型层面互不混用。 */
export type ArchiveId = string & { readonly __brand: 'ArchiveId' };
export type ScannedItemId = number & { readonly __brand: 'ScannedItemId' };

/** 当前扫描记录的匹配状态。 */
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

/** 前端会话内的扫描记录，ID 在清空后也不复用。 */
export interface ScannedItemRecord extends ScannedItem {
  scannedItemId: ScannedItemId;
  /** 后端自动纠错也会设置 correctedTitle，人工来源单独记录。 */
  manuallyCorrected: boolean;
}
