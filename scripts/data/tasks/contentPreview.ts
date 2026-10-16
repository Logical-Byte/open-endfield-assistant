/** 仅为人工指定的档案携带正文首行，避免输出全量正文。 */
export const CONTENT_PREVIEW_ARCHIVE_IDS = new Set([
  'nar_paper_map02_93_1',
  'nar_paper_map02_93_2',
]);

/** 源换行决定首行，移除显示标签，保留遮罩文字。 */
export function firstContentLine(text: string): string {
  return (
    text
      .replace(/<[^>]*>|＜[^＞]*＞/g, '')
      .split(/\r\n|\r|\n/)
      .map((line): string => line.trim())
      .find((line): boolean => !!line) ?? ''
  );
}
