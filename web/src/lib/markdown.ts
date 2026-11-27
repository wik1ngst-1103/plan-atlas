import DOMPurify from 'dompurify';
import { marked } from 'marked';

const FORBID_TAGS = [
  'style',
  'form',
  'input',
  'button',
  'textarea',
  'select',
  'iframe',
  'object',
  'embed',
];
const FORBID_ATTR = ['style'];

/** Relative paths (no scheme, not protocol-relative) and inline data images only. */
function localImage(src: string): boolean {
  const s = src.trim();
  if (/^data:image\//i.test(s)) return true;
  return !s.startsWith('//') && !s.startsWith('\\') && !/^[a-z][a-z0-9+.-]*:/i.test(s);
}

let hooked = false;

/**
 * Markdown → sanitized HTML. No styles, forms or embeds; remote images are
 * replaced by their alt text (no tracking pixels). External links open in a
 * new tab.
 */
export function renderMarkdown(md: string): string {
  if (!hooked) {
    hooked = true;
    DOMPurify.addHook('afterSanitizeAttributes', (node) => {
      if (node.tagName === 'A' && /^https?:/i.test(node.getAttribute('href') ?? '')) {
        node.setAttribute('target', '_blank');
        node.setAttribute('rel', 'noopener noreferrer');
      }
      if (node.tagName === 'IMG' && !localImage(node.getAttribute('src') ?? '')) {
        const alt = node.getAttribute('alt') ?? '';
        node.replaceWith(node.ownerDocument.createTextNode(alt));
      }
    });
  }
  const html = marked.parse(md, { async: false, gfm: true });
  return DOMPurify.sanitize(html, { FORBID_TAGS, FORBID_ATTR });
}
