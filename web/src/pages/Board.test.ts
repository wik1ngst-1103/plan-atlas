import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { boardFilter, defaultFilter } from '../lib/board';
import { drawer } from '../lib/drawer';
import type { Task } from '../lib/generated/Task';
import type { TaskRow } from '../lib/generated/TaskRow';
import { setLocale } from '../lib/i18n';
import { toastMsg } from '../lib/toast';
import Board from './Board.svelte';

const task = (id: string, name: string, status: Task['status'], tags: string[] = []): Task => ({
  id,
  name,
  complexity: 3,
  model: null,
  effort: null,
  depends_on: [],
  status,
  tags,
  body_md: '',
});
const rows: TaskRow[] = [
  {
    plan_id: 'PLAN-auth-001',
    plan_title: 'Migrate sessions to JWT',
    project: 'auth-gateway',
    plan_tags: ['domain/security'],
    task: task('TASK-001', 'Auth service', 'todo', ['tech/jwt']),
  },
  {
    plan_id: 'PLAN-auth-001',
    plan_title: 'Migrate sessions to JWT',
    project: 'auth-gateway',
    plan_tags: ['domain/security'],
    task: task('TASK-002', 'Token Management', 'in_progress'),
  },
  {
    plan_id: 'PLAN-viewer-001',
    plan_title: 'Point clouds',
    project: 'viewer-3d',
    plan_tags: [],
    task: task('TASK-001', 'Loader', 'review'),
  },
];

type Call = { url: string; method: string; body?: string };
let calls: Call[] = [];

function stubFetch(patch: (c: Call) => Response) {
  calls = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init?: RequestInit) => {
      const c = { url, method: init?.method ?? 'GET', body: init?.body as string | undefined };
      calls.push(c);
      if (c.method === 'PATCH') return patch(c);
      return new Response(JSON.stringify(url.startsWith('/api/tasks') ? rows : []));
    }),
  );
}

/** A DataTransfer stand-in (jsdom has none). */
function dataTransfer() {
  const data: Record<string, string> = {};
  return {
    setData: (k: string, v: string) => (data[k] = v),
    getData: (k: string) => data[k] ?? '',
    dropEffect: 'move',
    effectAllowed: 'move',
  };
}

const column = (name: RegExp) => screen.getByRole('region', { name });

beforeEach(() => {
  setLocale('en');
  boardFilter.set(defaultFilter());
  location.hash = '#/board';
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  drawer.close();
  toastMsg.set(null);
});

describe('Board', () => {
  it('drag_to_done_calls_patch', async () => {
    stubFetch(() => new Response('{}'));
    render(Board);
    const card = await screen.findByText('Auth service');
    const dt = dataTransfer();
    await fireEvent.dragStart(card.closest('[draggable="true"]')!, { dataTransfer: dt });
    await fireEvent.dragOver(column(/Done/), { dataTransfer: dt });
    await fireEvent.drop(column(/Done/), { dataTransfer: dt });

    const patch = calls.find((c) => c.method === 'PATCH');
    expect(patch?.url).toBe('/api/plans/PLAN-auth-001/tasks/TASK-001');
    expect(JSON.parse(patch!.body!)).toEqual({ status: 'done' });
    // Optimistic: the card is in Done at once.
    expect(within(column(/Done/)).getByText('Auth service')).toBeTruthy();
    await vi.waitFor(() => expect(get(toastMsg)?.text).toBe('Moved “Auth service” to Done'));
  });

  it('reverts a failed move and shows an error toast', async () => {
    stubFetch(() => new Response(JSON.stringify({ error: 'disk full' }), { status: 500 }));
    render(Board);
    const card = await screen.findByText('Auth service');
    const dt = dataTransfer();
    await fireEvent.dragStart(card.closest('[draggable="true"]')!, { dataTransfer: dt });
    await fireEvent.drop(column(/Done/), { dataTransfer: dt });
    await vi.waitFor(() => expect(get(toastMsg)?.error).toBe(true));
    expect(get(toastMsg)?.text).toContain('disk full');
    expect(within(column(/To do/)).getByText('Auth service')).toBeTruthy();
  });

  it('filter_count_text', async () => {
    stubFetch(() => new Response('{}'));
    render(Board);
    expect(await screen.findByText('Showing 3/3 tasks')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Clear filters' })).toBeNull();

    await fireEvent.change(screen.getByLabelText('Project'), { target: { value: 'auth-gateway' } });
    expect(screen.getByText('Showing 2/3 tasks')).toBeTruthy();
    await fireEvent.change(screen.getByLabelText('Topic'), { target: { value: 'tech/jwt' } });
    expect(screen.getByText('Showing 1/3 tasks')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect(screen.getByText('Showing 3/3 tasks')).toBeTruthy();
    expect(get(boardFilter)).toEqual(defaultFilter());
  });

  it('groups by domain without drag and drop', async () => {
    stubFetch(() => new Response('{}'));
    render(Board);
    await screen.findByText('Auth service');
    await fireEvent.change(screen.getByLabelText('Group by'), { target: { value: 'domain' } });
    expect(within(column(/security/)).getByText('Token Management')).toBeTruthy();
    expect(within(column(/Unclassified/)).getByText('Loader')).toBeTruthy();
    expect(screen.getByText(/only works when grouped by status/)).toBeTruthy();
    expect(document.querySelector('[draggable="true"]')).toBeNull();
  });

  it('card click opens the task drawer and keeps the board', async () => {
    stubFetch(() => new Response('{}'));
    render(Board);
    await fireEvent.click(await screen.findByText('Loader'));
    expect(location.hash).toBe('#/board');
    expect(get(drawer)).toEqual([{ type: 'task', plan: 'PLAN-viewer-001', task: 'TASK-001' }]);
  });
});
