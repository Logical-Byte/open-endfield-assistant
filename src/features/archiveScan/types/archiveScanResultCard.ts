import type { ArchiveAcquisitionMethod } from '@/features/gameData/types/archiveContract';

/** 档案扫描结果卡片的展示状态。 */
export enum ArchiveScanCardStatus {
  /** 已有成功匹配的扫描记录，匹配可能来自后端自动识别或人工纠错。 */
  Matched,
  /** OCR 得到了文字但未匹配，或人工输入的标题未匹配。 */
  Unrecognized,
  /** OCR 未得到文字，包括返回空文本或执行失败。 */
  OcrFailed,
  /** 当前没有匹配这份目录档案的扫描记录，不代表玩家实际未收集。 */
  NotMatched,
}

/** 档案扫描结果展示行：目录中的档案，或尚待纠错的扫描记录。 */
export interface ArchiveScanResultCardProps {
  status: ArchiveScanCardStatus;
  categoryLabel: string | null;
  candidates: string[];
  acquisitionMethod: ArchiveAcquisitionMethod | null;
  acquisitionLabel: string | null;
  oemUrl: string | null;
  intelUrl: string | null;
  imageUrl: string | null;
  title: string;
  archiveId: string | null;
  /** 对应的前端扫描记录 ID（未收集卡片为 null，不可纠错） */
  scannedItemId: number | null;
}
