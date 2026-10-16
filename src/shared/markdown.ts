import DOMPurify from 'dompurify';
import { marked } from 'marked';

// 消毒后统一处理超链接：一律在新标签页打开
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') {
    node.setAttribute('target', '_blank');
    node.setAttribute('rel', 'noopener noreferrer');
  }
});

/**
 * 将 Markdown 源码渲染为安全的 HTML 字符串。
 *
 * 渲染结果经过 DOMPurify 消毒，可安全地用于 `v-html`；
 * 所有超链接都会被强制加上 `target="_blank"` 与 `rel="noopener noreferrer"`。
 */
export function renderMarkdown(source: string): string {
  const html = marked.parse(source, { async: false }) as string;
  return DOMPurify.sanitize(html);
}
