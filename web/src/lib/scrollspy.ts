import type { Section } from './generated/Section';

/** A heading's top edge, in px, relative to the top of the visible scroll area. */
export type HeadingPos = { anchor: string; top: number };

/**
 * The section the reader is in: the lowest heading whose top has scrolled above `offset`
 * (px from the top of the visible area). Before the first heading, the first one; once the
 * reader has hit the bottom, the last heading that is on screen at all, so short closing
 * sections still get their turn. Order of `positions` does not matter.
 */
export function activeAnchor(
  positions: HeadingPos[],
  offset: number,
  atBottom = false,
  viewport = Infinity,
): string | null {
  if (!positions.length) return null;
  const sorted = [...positions].sort((a, b) => a.top - b.top);
  if (atBottom) {
    const onScreen = sorted.filter((p) => p.top < viewport);
    if (onScreen.length) return onScreen[onScreen.length - 1].anchor;
  }
  let current = sorted[0].anchor;
  for (const p of sorted) {
    if (p.top <= offset) current = p.anchor;
    else break;
  }
  return current;
}

/** One row of the "On this page" list. */
export type TocItem = {
  anchor: string;
  title: string;
  depth: number;
  /** Anchor of the top-level section this row belongs to (itself for top-level rows). */
  root: string;
  /** Rows under this one in the tree. */
  hasChildren: boolean;
};

/** The section tree as a flat list, in reading order. */
export function flattenToc(sections: Section[]): TocItem[] {
  const out: TocItem[] = [];
  const walk = (ss: Section[], depth: number, root: string | null) => {
    for (const s of ss) {
      const r = root ?? s.anchor;
      out.push({
        anchor: s.anchor,
        title: s.title,
        depth,
        root: r,
        hasChildren: s.children.length > 0,
      });
      walk(s.children, depth + 1, r);
    }
  };
  walk(sections, 0, null);
  return out;
}

/** Above this many rows the list folds sub-sections away, except in the section being read. */
export const COLLAPSE_OVER = 14;

/**
 * The rows to show. Short lists show everything; long ones show every top-level section plus
 * the sub-sections of the one being read (where `active` sits), so the list stays scannable.
 */
export function visibleToc(
  items: TocItem[],
  active: string | null,
  limit = COLLAPSE_OVER,
): TocItem[] {
  if (items.length <= limit) return items;
  const root = items.find((i) => i.anchor === active)?.root ?? null;
  return items.filter((i) => i.depth === 0 || i.root === root);
}

/** Anchors to mark: the active row and the top-level section that holds it. */
export function activeTrail(items: TocItem[], active: string | null): Set<string> {
  const item = items.find((i) => i.anchor === active);
  return new Set(item ? [item.anchor, item.root] : []);
}

/**
 * New scrollTop for a scrolling list so the row at [top, top+height) is visible with `margin`
 * to spare, or null when it already is.
 */
export function keepVisible(
  top: number,
  height: number,
  scrollTop: number,
  viewHeight: number,
  margin = 24,
): number | null {
  if (top - margin < scrollTop) return Math.max(0, top - margin);
  if (top + height + margin > scrollTop + viewHeight) return top + height + margin - viewHeight;
  return null;
}

/** 0…1: how far through `scrollHeight` the reader is. */
export function readProgress(scrollTop: number, scrollHeight: number, viewHeight: number): number {
  const max = scrollHeight - viewHeight;
  if (max <= 0) return 1;
  return Math.min(1, Math.max(0, scrollTop / max));
}

/** The element that scrolls `el` (the app's <main> on wide screens, the page on phones). */
export function scrollParent(el: Element | null): HTMLElement | null {
  for (let n = el?.parentElement ?? null; n; n = n.parentElement) {
    const o = getComputedStyle(n).overflowY;
    if ((o === 'auto' || o === 'scroll') && n.scrollHeight > n.clientHeight) return n;
  }
  return null;
}
