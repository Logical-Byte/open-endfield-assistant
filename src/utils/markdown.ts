import DOMPurify from 'dompurify';
import { marked } from 'marked';

/**
 * 将来源提供的 Markdown 转为可插入页面的安全 HTML。
 *
 * `marked` + `DOMPurify` 的组合改编自 PR #6；必须先解析再清洗，避免更新日志注入脚本。
 */
export function renderMarkdown(markdown: string): string {
  return DOMPurify.sanitize(marked.parse(markdown, { async: false }));
}
