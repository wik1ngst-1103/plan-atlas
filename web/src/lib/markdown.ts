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

const HTML_NS = 'http://www.w3.org/1999/xhtml';

let hooked = false;

/**
 * Markdown → sanitized HTML. No styles, forms or embeds; remote images are
 * replaced by their alt text (no tracking pixels). External links open in a
 * new tab. Tables come wrapped in `div.table-wrap`.
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
  const clean = DOMPurify.sanitize(html, { FORBID_TAGS, FORBID_ATTR, RETURN_DOM_FRAGMENT: true });
  // Wrap HTML tables on the sanitized tree, then serialize once: wide tables scroll inside
  // the wrapper, not the page. (SVG/MathML elements named "table" are left alone.)
  for (const table of clean.querySelectorAll('table')) {
    if (table.namespaceURI !== HTML_NS) continue;
    const wrap = document.createElement('div');
    wrap.className = 'table-wrap';
    table.replaceWith(wrap);
    wrap.append(table);
  }
  const out = document.createElement('div');
  out.append(clean);
  return out.innerHTML;
}
