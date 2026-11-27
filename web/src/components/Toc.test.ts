import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Section } from '../lib/generated/Section';
import { setLocale } from '../lib/i18n';
import { flattenToc } from '../lib/scrollspy';
import Toc from './Toc.svelte';

const s = (title: string, anchor: string, children: Section[] = [], level = 2): Section => ({
  level,
  title,
  anchor,
  body_md: '',
  children,
});

const long = 'New file: server/src/agents/proof/proof.compiler.ts';
const items = flattenToc([
  s('Context', 'context'),
  s('Phase 1 — Proof-First claim→proof layer', 'phase-1', [
    s(long, 'proof-compiler', [], 3),
    s('server/src/agents/agent.types.ts', 'agent-types', [], 3),
  ]),
  s('01. Overview', 'overview'),
]);

afterEach(cleanup);

describe('Toc', () => {
  it('highlights the active row and marks its parent section', () => {
    setLocale('en');
    const { container } = render(Toc, { items, active: 'proof-compiler', onGo: () => {} });
    const on = container.querySelector('button.on');
    expect(on?.getAttribute('data-anchor')).toBe('proof-compiler');
    expect(on?.getAttribute('aria-current')).toBe('location');
    expect(container.querySelector('button.trail')?.getAttribute('data-anchor')).toBe('phase-1');
    expect(container.querySelectorAll('button.on')).toHaveLength(1);
    // "2 / 3": the reader is in the second of three top-level sections.
    expect(container.querySelector('.count')?.textContent?.trim()).toBe('2 / 3');
  });

  it('shortens file-path rows to the file name, full heading in the tooltip', () => {
    const { container } = render(Toc, { items, active: 'context', onGo: () => {} });
    const row = container.querySelector<HTMLElement>('[data-anchor="proof-compiler"]')!;
    expect(row.textContent?.trim()).toBe('proof.compiler.ts');
    expect(row.getAttribute('title')).toBe(long);
    expect(row.classList.contains('mono')).toBe(true);
    for (const b of container.querySelectorAll('button')) {
      expect(b.getAttribute('title')).toBeTruthy();
    }
  });

  it('shows standard sections as number + name', () => {
    const { container } = render(Toc, { items, active: null, onGo: () => {} });
    const row = container.querySelector('[data-anchor="overview"]')!;
    expect(row.querySelector('.n')?.textContent).toBe('01');
    expect(row.textContent?.replace(/\s+/g, ' ').trim()).toBe('01Overview');
  });

  it('calls onGo with the anchor and never changes the route', async () => {
    location.hash = '#/plan/P/docs';
    const onGo = vi.fn();
    const { container } = render(Toc, { items, active: null, onGo });
    await fireEvent.click(container.querySelector('[data-anchor="agent-types"]')!);
    expect(onGo).toHaveBeenCalledWith('agent-types');
    expect(location.hash).toBe('#/plan/P/docs');
  });

  it('folds sub-sections of other sections when the list is long', () => {
    const many = flattenToc(
      Array.from({ length: 8 }, (_, i) =>
        s(`Part ${i}`, `part-${i}`, [
          s(`Sub ${i}a`, `sub-${i}a`, [], 3),
          s(`Sub ${i}b`, `sub-${i}b`, [], 3),
        ]),
      ),
    );
    const { container } = render(Toc, { items: many, active: 'sub-3b', onGo: () => {} });
    const shown = [...container.querySelectorAll('button')].map((b) =>
      b.getAttribute('data-anchor'),
    );
    expect(shown).toHaveLength(10);
    expect(shown).toContain('sub-3a');
    expect(shown).not.toContain('sub-4a');
  });
});
