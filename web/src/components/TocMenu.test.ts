import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { Section } from '../lib/generated/Section';
import { setLocale } from '../lib/i18n';
import { flattenToc } from '../lib/scrollspy';
import TocMenu from './TocMenu.svelte';

const s = (title: string, anchor: string, children: Section[] = []): Section => ({
  level: 2,
  title,
  anchor,
  body_md: '',
  children,
});
const items = flattenToc([
  s('Phase 1', 'phase-1', [
    s('server/src/agents/creative-pipeline.service.ts (wire it)', 'cps'),
    s('server/src/agents/scene.agent.ts', 'scene'),
  ]),
  s('02. Requirements & Scope', 'req'),
]);

afterEach(cleanup);

const setup = (active: string | null) => {
  setLocale('en');
  const r = render(TocMenu, { items, active, onGo: () => {}, open: true });
  const details = r.container.querySelector('details')!;
  return { ...r, details, summary: details.querySelector('summary')! };
};

describe('TocMenu', () => {
  it('names the current section the way the list does', () => {
    expect(setup('cps').summary.querySelector('.tm-current')?.textContent).toBe(
      'creative-pipeline.service.ts',
    );
    cleanup();
    expect(setup('req').summary.querySelector('.tm-current')?.textContent).toBe(
      'Requirements & Scope',
    );
  });

  it('Escape closes it and returns focus to the bar', async () => {
    const { details, summary } = setup('scene');
    expect(details.open).toBe(true);
    await fireEvent.keyDown(document, { key: 'Escape' });
    flushSync();
    expect(details.open).toBe(false);
    expect(document.activeElement).toBe(summary);
  });

  it('a tap outside closes it, a tap inside does not', async () => {
    const { details } = setup('scene');
    await fireEvent.pointerDown(details.querySelector('button')!);
    flushSync();
    expect(details.open).toBe(true);
    await fireEvent.pointerDown(document.body);
    flushSync();
    expect(details.open).toBe(false);
  });
});
