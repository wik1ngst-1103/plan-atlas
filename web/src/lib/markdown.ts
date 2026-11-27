import DOMPurify from 'dompurify';
import { marked } from 'marked';

let hooked = false;

/** Markdown → sanitized HTML. External links open in a new tab. */
export function renderMarkdown(md: string): string {
  if (!hooked) {
    hooked = true;
    DOMPurify.addHook('afterSanitizeAttributes', (node) => {
      if (node.tagName === 'A' && /^https?:/i.test(node.getAttribute('href') ?? '')) {
        node.setAttribute('target', '_blank');
        node.setAttribute('rel', 'noopener noreferrer');
      }
    });
  }
  const html = marked.parse(md, { async: false, gfm: true });
  return DOMPurify.sanitize(html);
}
