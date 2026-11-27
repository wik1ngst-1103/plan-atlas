import { describe, expect, it } from 'vitest';
import type { Section } from './generated/Section';
import { renderMarkdown } from './markdown';
import { formatDate, planStatus, uniqueAnchors } from './plans';

const sec = (anchor: string, children: Section[] = []): Section => ({
  level: 2,
  title: anchor,
  anchor,
  body_md: '',
  children,
});

describe('plans helpers', () => {
  it('derives the plan status from its tasks', () => {
    expect(planStatus(0, 0, false)).toBeNull();
    expect(planStatus(0, 3, false)).toBe('todo');
    expect(planStatus(0, 3, true)).toBe('in_progress');
    expect(planStatus(1, 3, false)).toBe('in_progress');
    expect(planStatus(3, 3, false)).toBe('done');
  });

  it('makes duplicate heading anchors unique', () => {
    const out = uniqueAnchors([sec('notes', [sec('notes')]), sec('notes')]);
    expect([out[0].anchor, out[0].children[0].anchor, out[1].anchor]).toEqual([
      'notes',
      'notes-2',
      'notes-3',
    ]);
  });

  it('formats dates without a timezone shift', () => {
    expect(formatDate('2026-11-20', 'en')).toContain('20');
    expect(formatDate('2026-11-20', 'en')).toContain('2026');
    expect(formatDate('not a date', 'en')).toBe('not a date');
  });

  it('renders markdown and strips scripts', () => {
    const html = renderMarkdown(
      '**bold** <script>alert(1)</script><img src=x onerror=alert(1)> [x](https://e.com)',
    );
    expect(html).toContain('<strong>bold</strong>');
    expect(html).not.toContain('<script');
    expect(html).not.toContain('onerror');
    expect(html).toContain('target="_blank"');
  });
});
