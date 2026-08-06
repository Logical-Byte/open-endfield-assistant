export type ScanResultStatus = 'success' | 'unrecognized' | 'failed';

/** 单份档案的扫描结果（与 Rust 侧 `ScanResult` 对齐）。 */
export interface ScanResult {
  /** 识别状态：success（纠错成功）| unrecognized（识别到文本但无法纠错）| failed（OCR 为空） */
  status: ScanResultStatus;
  /** 全局序号（从 1 开始，跨分类连续递增） */
  index: number;
  /** 档案库大类 id（pageType：multi_media / text / document） */
  category: string;
  /** 档案库小类 id（categoryId） */
  sub_category: string;
  /** 档案详情页面截图（base64 PNG data URL） */
  image: string;
  /** OCR 识别结果（前端可编辑） */
  ocr_result: string;
  /** 纠错后的档案标题（无法识别时为 null） */
  corrected_title: string | null;
  /** 纠错命中的档案 id（allItems 的 id，同标题多条时返回全部） */
  item_ids: string[];
}

export enum CollectType {
  Collected,
  Unrecognized,
  Failed,
  NotCollected,
}

export interface ScanResultCardProps {
  collectType: CollectType;
  category: string;
  subCategory: string;
  imageUrl: string | null;
  title: string;
}
