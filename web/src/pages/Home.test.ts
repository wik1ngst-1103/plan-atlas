import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PlanSummary } from '../lib/generated/PlanSummary';
import { setLocale } from '../lib/i18n';
import Home from './Home.svelte';

const plan: PlanSummary = {
  id: 'PLAN-a-001',
  project: 'a',
  title: 'First plan',
  tags: [],
  created: null,
  task_count: 1,
  done_count: 0,
  structured: true,
  warnings_count: 0,
  tasks: [{ id: 'TASK-001', name: 'Do it', status: 'todo' }],
};

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('Home', () => {
  it('home_loads_plans_without_fetching_tasks', async () => {
    setLocale('en');
    const urls: string[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) => {
        urls.push(url);
        const body = url.startsWith('/api/plans') ? [plan] : [];
        return new Response(JSON.stringify(body));
      }),
    );
    render(Home);
    expect(await screen.findByText('First plan')).toBeTruthy();
    expect(urls).toContain('/api/plans');
    expect(urls.filter((u) => u.startsWith('/api/tasks'))).toEqual([]);
    expect(urls.filter((u) => u.startsWith('/api/graph'))).toEqual([]);
  });
});
