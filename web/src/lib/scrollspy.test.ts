import { describe, expect, it } from 'vitest';
import type { Section } from './generated/Section';
import {
  activeAnchor,
  activeTrail,
  flattenToc,
  keepVisible,
  readProgress,
  visibleToc,
} from './scrollspy';

const sec = (anchor: string, children: Section[] = [], level = 2): Section => ({
  level,
  title: anchor.toUpperCase(),
  anchor,
  body_md: '',
  children,
});

describe('activeAnchor', () => {
  const pos = [
    { anchor: 'a', top: -900 },
    { anchor: 'b', top: -200 },
    { anchor: 'c', top: 300 },
    { anchor: 'd', top: 1400 },
  ];

  it('picks the last heading scrolled past the offset', () => {
    expect(activeAnchor(pos, 120)).toBe('b');
    expect(activeAnchor(pos, 300)).toBe('c');
  });

  it('ignores the order headings are given in', () => {
    expect(activeAnchor([...pos].reverse(), 120)).toBe('b');
  });

  it('falls back to the first heading before any is reached', () => {
    expect(
      activeAnchor(
        [
          { anchor: 'x', top: 400 },
          { anchor: 'y', top: 900 },
        ],
        120,
      ),
    ).toBe('x');
  });

  it('at the bottom, gives short closing sections their turn', () => {
    expect(activeAnchor(pos, 120, true, 1000)).toBe('c');
    expect(activeAnchor(pos, 120, true, 2000)).toBe('d');
  });

  it('is null with no headings', () => {
    expect(activeAnchor([], 120)).toBeNull();
  });
});

describe('toc tree', () => {
  const tree = [
    sec('intro'),
    sec('phase-1', [sec('p1-a', [], 3), sec('p1-b', [sec('p1-b-x', [], 4)], 3)]),
    sec('phase-2', [sec('p2-a', [], 3)]),
  ];
  const items = flattenToc(tree);

  it('flattens in reading order with depth and top-level root', () => {
    expect(items.map((i) => [i.anchor, i.depth, i.root])).toEqual([
      ['intro', 0, 'intro'],
      ['phase-1', 0, 'phase-1'],
      ['p1-a', 1, 'phase-1'],
      ['p1-b', 1, 'phase-1'],
      ['p1-b-x', 2, 'phase-1'],
      ['phase-2', 0, 'phase-2'],
      ['p2-a', 1, 'phase-2'],
    ]);
    expect(items[1].hasChildren).toBe(true);
    expect(items[0].hasChildren).toBe(false);
  });

  it('short lists show everything', () => {
    expect(visibleToc(items, 'intro')).toHaveLength(items.length);
  });

  it('long lists fold sub-sections except in the section being read', () => {
    const shown = visibleToc(items, 'p1-b-x', 4).map((i) => i.anchor);
    expect(shown).toEqual(['intro', 'phase-1', 'p1-a', 'p1-b', 'p1-b-x', 'phase-2']);
    expect(visibleToc(items, 'phase-2', 4).map((i) => i.anchor)).toEqual([
      'intro',
      'phase-1',
      'phase-2',
      'p2-a',
    ]);
    expect(visibleToc(items, null, 4).map((i) => i.anchor)).toEqual([
      'intro',
      'phase-1',
      'phase-2',
    ]);
  });

  it('marks the active row and its top-level section', () => {
    expect([...activeTrail(items, 'p1-b-x')].sort()).toEqual(['p1-b-x', 'phase-1']);
    expect([...activeTrail(items, 'intro')]).toEqual(['intro']);
    expect(activeTrail(items, 'nope').size).toBe(0);
  });
});

describe('keepVisible', () => {
  it('leaves a visible row alone', () => {
    expect(keepVisible(100, 20, 0, 400)).toBeNull();
  });
  it('scrolls up to a row above the view', () => {
    expect(keepVisible(50, 20, 200, 400)).toBe(26);
    expect(keepVisible(10, 20, 200, 400)).toBe(0);
  });
  it('scrolls down to a row below the view', () => {
    expect(keepVisible(500, 20, 0, 400)).toBe(144);
  });
});

describe('readProgress', () => {
  it('is the scrolled fraction, clamped', () => {
    expect(readProgress(0, 2000, 1000)).toBe(0);
    expect(readProgress(500, 2000, 1000)).toBe(0.5);
    expect(readProgress(5000, 2000, 1000)).toBe(1);
    expect(readProgress(0, 800, 1000)).toBe(1);
  });
});
