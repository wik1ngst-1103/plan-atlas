import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { boardFilter, defaultFilter } from '../lib/board';
import { drawer } from '../lib/drawer';
import type { Task } from '../lib/generated/Task';
import type { TaskRow } from '../lib/generated/TaskRow';
import { setLocale } from '../lib/i18n';
import Board from '../pages/Board.svelte';
import TopicPanel from './TopicPanel.svelte';

const task = (id: string, tags: string[] = []): Task => ({
  id,
  name: `Task ${id}`,
  complexity: 2,
  model: null,
  effort: null,
  depends_on: [],
  status: 'todo',
  tags,
  body_md: '',
});
// domain/security is on the plan only; none of its tasks carry it.
const rows: TaskRow[] = [
  {
    plan_id: 'PLAN-a-001',
    plan_title: 'Secure plan',
    project: 'a',
    plan_tags: ['domain/security'],
    task: task('TASK-001'),
  },
  {
    plan_id: 'PLAN-a-001',
    plan_title: 'Secure plan',
    project: 'a',
    plan_tags: ['domain/security'],
    task: task('TASK-002', ['tech/jwt']),
  },
  {
    plan_id: 'PLAN-b-001',
    plan_title: 'Other plan',
    project: 'b',
    plan_tags: [],
    task: task('TASK-001'),
  },
];

beforeEach(() => {
  setLocale('en');
  boardFilter.set(defaultFilter());
  location.hash = '#/board';
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => {
      const u = new URL(url, 'http://x');
      const tag = u.searchParams.get('tag');
      let body: unknown = [];
      // Server rule R13: a task has a topic directly or through its plan.
      if (u.pathname === '/api/tasks')
        body = tag
          ? rows.filter((r) => r.task.tags.includes(tag) || r.plan_tags.includes(tag))
          : rows;
      if (u.pathname === '/api/plans' && tag === 'domain/security')
        body = [
          {
            id: 'PLAN-a-001',
            project: 'a',
            title: 'Secure plan',
            tags: [tag],
            created: null,
            task_count: 2,
            done_count: 0,
            structured: true,
            warnings_count: 0,
            tasks: [],
          },
        ];
      return new Response(JSON.stringify(body));
    }),
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  drawer.close();
});

describe('TopicPanel', () => {
  it('task count matches the board filtered by a plan-only topic', async () => {
    const panel = render(TopicPanel, { tag: 'domain/security' });
    const stat = await screen.findByText('tasks', { selector: '.stat span' });
    const panelCount = Number(stat.previousElementSibling!.textContent);
    expect(panelCount).toBe(2);

    await fireEvent.click(screen.getByRole('button', { name: /Filter the task board/ }));
    panel.unmount();
    render(Board);
    expect(await screen.findByText(`Showing ${panelCount}/3 tasks`)).toBeTruthy();
  });
});
