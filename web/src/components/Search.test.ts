import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { drawer } from '../lib/drawer';
import type { PlanSummary } from '../lib/generated/PlanSummary';
import type { SearchResults } from '../lib/generated/SearchResults';
import type { Task } from '../lib/generated/Task';
import { setLocale } from '../lib/i18n';
import Search from './Search.svelte';

const plan = (id: string, title: string, created: string): PlanSummary => ({
  id,
  project: 'auth-gateway',
  title,
  tags: [],
  created,
  task_count: 2,
  done_count: 1,
  structured: true,
  warnings_count: 0,
  tasks: [],
});
const task = (id: string, name: string): Task => ({
  id,
  name,
  complexity: 2,
  model: null,
  effort: null,
  depends_on: [],
  status: 'todo',
  tags: [],
  body_md: '',
});
const results = (title: string): SearchResults => ({
  plans: [plan('PLAN-auth-gateway-001', title, '2026-11-20')],
  tasks: [
    {
      plan_id: 'PLAN-auth-gateway-001',
      plan_title: title,
      project: 'auth-gateway',
      plan_tags: [],
      task: task('TASK-002', 'Đăng nhập bằng token'),
    },
  ],
  topics: [{ tag: 'domain/security', plans: 1, tasks: 2 }],
});

type Pending = { q: string; resolve: (body: unknown) => void };
let pending: Pending[] = [];

beforeEach(() => {
  setLocale('en');
  location.hash = '#/map';
  pending = [];
  vi.stubGlobal(
    'fetch',
    vi.fn((url: string) => {
      const u = new URL(url, 'http://x');
      if (u.pathname === '/api/search') {
        return new Promise<Response>((resolve) =>
          pending.push({
            q: u.searchParams.get('q') ?? '',
            resolve: (b) => resolve(new Response(JSON.stringify(b))),
          }),
        );
      }
      if (u.pathname === '/api/plans')
        return Promise.resolve(
          new Response(
            JSON.stringify([
              plan('PLAN-a-001', 'Old plan', '2026-10-01'),
              plan('PLAN-a-002', 'Newest plan', '2026-11-25'),
            ]),
          ),
        );
      if (u.pathname === '/api/topics')
        return Promise.resolve(
          new Response(
            JSON.stringify([
              {
                tag: 'domain/security',
                group: 'domain',
                label: 'Security',
                desc: 'Keeping things safe.',
                plans: 1,
                tasks: 2,
              },
            ]),
          ),
        );
      return Promise.resolve(new Response('[]'));
    }),
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.useRealTimers();
  drawer.close();
});

const settle = () => new Promise((r) => setTimeout(r, 0));

async function typeQuery(input: HTMLElement, q: string) {
  await fireEvent.input(input, { target: { value: q } });
  await vi.advanceTimersByTimeAsync(130);
}

describe('Search', () => {
  it('search_keyboard_enter_opens_selected', async () => {
    vi.useFakeTimers();
    render(Search);
    const input = screen.getByRole('combobox');
    await fireEvent.focus(input);
    await typeQuery(input, 'dang nhap');
    expect(pending.map((p) => p.q)).toEqual(['dang nhap']);
    pending[0].resolve(results('Đăng nhập một lần'));
    await vi.advanceTimersByTimeAsync(0);

    // Groups with counts; the plan title is highlighted without accents.
    expect(screen.getByText('Plans (1)')).toBeTruthy();
    expect(screen.getByText('Tasks (1)')).toBeTruthy();
    expect(screen.getByText('Topics (1)')).toBeTruthy();
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(3);
    expect(options[0].querySelector('b')!.innerHTML).toBe(
      '<mark>Đăng</mark> <mark>nhập</mark> một lần',
    );
    expect(options[1].textContent!.replace(/\u00a0/g, ' ')).toContain(
      '📄 Đăng nhập một lần › TASK-002',
    );
    expect(options[2].textContent).toContain('Domain · 1 plan');
    expect(options[0].getAttribute('aria-selected')).toBe('true');

    // ↓ selects the task; Enter opens it in the drawer without navigating.
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(options[1].getAttribute('aria-selected')).toBe('true');
    expect(input.getAttribute('aria-activedescendant')).toBe(options[1].id);
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(get(drawer)).toEqual([
      { type: 'task', plan: 'PLAN-auth-gateway-001', task: 'TASK-002' },
    ]);
    expect(location.hash).toBe('#/map');
    expect(screen.queryByRole('listbox')).toBeNull();
    expect((input as HTMLInputElement).value).toBe('');

    // A plan result navigates to the plan document.
    await fireEvent.focus(input);
    await typeQuery(input, 'dang');
    pending[1].resolve(results('Đăng nhập một lần'));
    await vi.advanceTimersByTimeAsync(0);
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    await fireEvent.keyDown(input, { key: 'ArrowUp' });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(location.hash).toBe('#/plan/PLAN-auth-gateway-001/docs');
  });

  it('search_ignores_stale_responses', async () => {
    vi.useFakeTimers();
    render(Search);
    const input = screen.getByRole('combobox');
    await fireEvent.focus(input);
    await typeQuery(input, 'tok');
    await typeQuery(input, 'token');
    expect(pending.map((p) => p.q)).toEqual(['tok', 'token']);
    // The newer answer arrives first, then the older one: the older one is dropped.
    pending[1].resolve(results('Token rotation'));
    await vi.advanceTimersByTimeAsync(0);
    pending[0].resolve(results('Tokyo trip'));
    await vi.advanceTimersByTimeAsync(0);
    expect(screen.getAllByRole('option')[0].textContent).toContain('Token rotation');
    expect(screen.queryByText(/Tokyo/)).toBeNull();
  });

  it('typing fast sends one request (120 ms debounce)', async () => {
    vi.useFakeTimers();
    render(Search);
    const input = screen.getByRole('combobox');
    await fireEvent.focus(input);
    for (const q of ['b', 'ba', 'bao']) {
      await fireEvent.input(input, { target: { value: q } });
      await vi.advanceTimersByTimeAsync(50);
    }
    expect(pending).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(80);
    expect(pending.map((p) => p.q)).toEqual(['bao']);
  });

  it('empty query lists recently created plans; no results shows a hint; Esc closes', async () => {
    render(Search);
    const input = screen.getByRole('combobox');
    await fireEvent.focus(input);
    await settle();
    await settle();
    expect(screen.getByText('Recently created plans')).toBeTruthy();
    const options = screen.getAllByRole('option');
    expect(options[0].textContent).toContain('Newest plan');
    expect(options[1].textContent).toContain('Old plan');

    await fireEvent.input(input, { target: { value: 'zzz' } });
    await new Promise((r) => setTimeout(r, 140));
    pending[0].resolve({ plans: [], tasks: [], topics: [] });
    await settle();
    expect(screen.getByText(/No results for “zzz”/)).toBeTruthy();

    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).toBeNull();
    expect((input as HTMLInputElement).value).toBe('');
  });

  it('"/" and Ctrl+K focus the box unless typing elsewhere; a click outside closes it', async () => {
    render(Search);
    const input = screen.getByRole('combobox');
    const other = document.createElement('input');
    document.body.appendChild(other);
    other.focus();
    await fireEvent.keyDown(window, { key: '/' });
    expect(document.activeElement).toBe(other);
    other.blur();
    await fireEvent.keyDown(window, { key: '/' });
    expect(document.activeElement).toBe(input);
    input.blur();
    other.focus();
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(document.activeElement).toBe(input);
    await settle();
    expect(screen.getByRole('listbox')).toBeTruthy();
    await fireEvent.pointerDown(document.body);
    expect(screen.queryByRole('listbox')).toBeNull();
    other.remove();
  });
});
