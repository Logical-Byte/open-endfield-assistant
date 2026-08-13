import DOMPurify from 'dompurify';
import { marked } from 'marked';

/**
 * 将 Markdown 源码渲染为安全的 HTML 字符串。
 *
 * 渲染结果经过 DOMPurify 消毒，可安全地用于 `v-html`。
 */
export function renderMarkdown(source: string): string {
  const html = marked.parse(source, { async: false }) as string;
  return DOMPurify.sanitize(html);
}
