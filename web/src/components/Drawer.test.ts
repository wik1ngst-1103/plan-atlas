import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { drawer } from '../lib/drawer';
import { setLocale } from '../lib/i18n';
import Drawer from './Drawer.svelte';

/** `inert` as attribute (browsers) or property (jsdom has no reflection). */
const isInert = (el: HTMLElement) =>
  el.hasAttribute('inert') || (el as HTMLElement & { inert?: boolean }).inert === true;

beforeEach(() => {
  setLocale('en');
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response('[]')),
  );
});
afterEach(() => {
  cleanup();
  drawer.close();
  vi.unstubAllGlobals();
});

describe('Drawer', () => {
  it('esc_closes_drawer', async () => {
    const opener = document.createElement('button');
    document.body.appendChild(opener);
    opener.focus();
    render(Drawer);
    const aside = screen.getByLabelText('Details', { selector: 'aside' });
    expect(isInert(aside)).toBe(true);

    drawer.open({ type: 'activity' });
    await tick();
    expect(isInert(aside)).toBe(false);
    expect(aside.contains(document.activeElement)).toBe(true);

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(get(drawer)).toEqual([]);
    await tick();
    expect(isInert(aside)).toBe(true);
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });

  it('shows Back only inside a stack', async () => {
    render(Drawer);
    drawer.open({ type: 'activity' });
    await tick();
    expect(screen.queryByRole('button', { name: /Back/ })).toBeNull();
    drawer.open({ type: 'topic', tag: 'tech/jwt' }, { push: true });
    await tick();
    await fireEvent.click(screen.getByRole('button', { name: /Back/ }));
    expect(get(drawer)).toEqual([{ type: 'activity' }]);
  });

  it('"Open full plan" closes the drawer and navigates', async () => {
    const detail = {
      id: 'PLAN-a-001',
      project: 'a',
      title: 'First plan',
      tags: [],
      related: [],
      created: null,
      sections: [],
      tasks: [],
      structured: true,
      source_path: '/x.md',
      content_hash: 'h',
      warnings: [],
      incoming: [],
      outgoing_titles: {},
    };
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async (url: string) =>
          new Response(JSON.stringify(url.startsWith('/api/plans/') ? detail : [])),
      ),
    );
    location.hash = '#/board';
    render(Drawer);
    drawer.open({ type: 'plan', plan: 'PLAN-a-001' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Open full plan ›' }));
    expect(get(drawer)).toEqual([]);
    expect(location.hash).toBe('#/plan/PLAN-a-001/docs');
    location.hash = '';
  });
});
