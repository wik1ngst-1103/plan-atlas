import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { drawer, showRaw } from '../lib/drawer';
import type { LogEntry } from '../lib/generated/LogEntry';
import type { PlanDetail } from '../lib/generated/PlanDetail';
import { setLocale } from '../lib/i18n';
import { activity } from '../lib/live';
import { toastMsg } from '../lib/toast';
import TaskPanel from './TaskPanel.svelte';

const detail: PlanDetail = {
  id: 'PLAN-auth-001',
  project: 'auth-gateway',
  title: 'Migrate sessions to JWT',
  tags: [],
  related: [],
  created: null,
  sections: [],
  tasks: [
    {
      id: 'TASK-001',
      name: 'Auth service',
      complexity: 2,
      model: null,
      effort: null,
      depends_on: [],
      status: 'done',
      tags: [],
      body_md: '',
    },
    {
      id: 'TASK-002',
      name: 'Token Management',
      complexity: 3,
      model: null,
      effort: null,
      depends_on: ['TASK-001'],
      status: 'in_progress',
      tags: [],
      body_md: 'Rotate **refresh** tokens.<script>alert(1)</script>',
    },
  ],
  structured: true,
  source_path: '/x.md',
  content_hash: 'h',
  warnings: [],
  incoming: [],
  outgoing_titles: {},
};
const log: LogEntry[] = [
  {
    at: 1,
    role: 'Explore',
    step: {
      icon: '🔍',
      text_en: 'search the code',
      text_vi: 'tìm kiếm trong code',
      raw: 'Grep jwt',
    },
  },
];
let patches: { url: string; body: string }[] = [];

beforeEach(() => {
  patches = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init?: RequestInit) => {
      if (init?.method === 'PATCH') {
        patches.push({ url, body: init.body as string });
        const tasks = detail.tasks.map((x) =>
          x.id === 'TASK-002' ? { ...x, status: 'done' as const } : x,
        );
        return new Response(JSON.stringify({ ...detail, tasks }));
      }
      if (url.includes('/log')) return new Response(JSON.stringify(log));
      if (url.startsWith('/api/plans/')) return new Response(JSON.stringify(detail));
      return new Response('[]');
    }),
  );
  activity.set([
    {
      plan_id: 'PLAN-auth-001',
      task_id: 'TASK-002',
      agents: [
        {
          agent_id: 'a1',
          role: 'main',
          model: 'claude-sonnet-4-5',
          step: {
            icon: '📖',
            text_en: 'read token.rs',
            text_vi: 'đọc token.rs',
            raw: 'Read /src/token.rs',
          },
          at: 2,
        },
      ],
    },
  ]);
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  activity.set([]);
  showRaw.set(false);
  drawer.close();
  toastMsg.set(null);
});

describe('TaskPanel', () => {
  it('explains live work, history, difficulty and description', async () => {
    setLocale('en');
    render(TaskPanel, { plan: 'PLAN-auth-001', task: 'TASK-002' });
    expect(await screen.findByRole('heading', { name: 'Token Management' })).toBeTruthy();
    expect(screen.getByText('📖 is reading token.rs')).toBeTruthy();
    expect(await screen.findByText(/Research assistant\s+searched the code/)).toBeTruthy();
    expect(screen.getByText(/→ Claude uses/).textContent).toBe(
      '→ Claude uses Sonnet, balanced speed and quality, thinking: moderate',
    );
    expect(screen.getByText('refresh').tagName).toBe('STRONG');
    expect(document.querySelector('script')).toBeNull();

    expect(screen.queryByText('Grep jwt')).toBeNull();
    await fireEvent.click(screen.getByLabelText('Show technical details (for developers)'));
    expect(screen.getByText('Grep jwt')).toBeTruthy();
    expect(screen.getByText('Read /src/token.rs')).toBeTruthy();
  });

  it('uses past tense in Vietnamese logs', async () => {
    setLocale('vi');
    render(TaskPanel, { plan: 'PLAN-auth-001', task: 'TASK-002' });
    expect(await screen.findByText(/Trợ lý tìm hiểu\s+đã tìm kiếm trong code/)).toBeTruthy();
    expect(screen.getByText('📖 đang đọc token.rs')).toBeTruthy();
  });

  it('status buttons patch and toast; dependencies push the drawer', async () => {
    setLocale('en');
    drawer.open({ type: 'task', plan: 'PLAN-auth-001', task: 'TASK-002' });
    render(TaskPanel, { plan: 'PLAN-auth-001', task: 'TASK-002' });
    await fireEvent.click(await screen.findByRole('button', { name: '● Done' }));
    await vi.waitFor(() => expect(get(toastMsg)?.text).toBe('Moved “Token Management” to Done'));
    expect(patches[0].url).toBe('/api/plans/PLAN-auth-001/tasks/TASK-002');
    expect(JSON.parse(patches[0].body)).toEqual({ status: 'done' });
    expect(screen.getByRole('button', { name: '● Done' }).getAttribute('aria-pressed')).toBe(
      'true',
    );

    await fireEvent.click(screen.getByRole('button', { name: 'Auth service' }));
    expect(get(drawer)).toHaveLength(2);
    expect(get(drawer)[1]).toEqual({ type: 'task', plan: 'PLAN-auth-001', task: 'TASK-001' });
  });
});
