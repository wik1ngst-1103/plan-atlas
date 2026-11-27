/**
 * Search text helpers. `norm` removes accents one character at a time and keeps
 * the length, so a match found in the normalised text maps back to the same
 * positions in the original ("bao mat" highlights "Bảo mật").
 */
export const norm = (s: string): string =>
  [...String(s)]
    .map((c) => c.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[đĐ]/, 'd').toLowerCase()[0] || ' ')
    .join('');

/** Normalised, non-empty words of a query. */
export const searchWords = (q: string): string[] => norm(q).trim().split(/\s+/).filter(Boolean);

const ESC: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

/** HTML-escaped `text` with every occurrence of the (normalised) `words` wrapped in <mark>. */
export function hl(text: string, words: string[]): string {
  const chars = [...String(text)];
  const n = norm(text);
  const marked = new Array<boolean>(chars.length).fill(false);
  for (const w of words) {
    if (!w) continue;
    for (let i = n.indexOf(w); i > -1; i = n.indexOf(w, i + 1)) {
      for (let k = i; k < i + w.length; k++) marked[k] = true;
    }
  }
  let out = '';
  let open = false;
  chars.forEach((c, i) => {
    if (marked[i] !== open) {
      out += open ? '</mark>' : '<mark>';
      open = marked[i];
    }
    out += ESC[c] ?? c;
  });
  return open ? `${out}</mark>` : out;
}
