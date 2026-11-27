import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, describe, expect, it } from 'vitest';
import { drawer } from '../lib/drawer';
import type { LiveTask } from '../lib/generated/LiveTask';
import type { PlanSummary } from '../lib/generated/PlanSummary';
import type { Task } from '../lib/generated/Task';
import { setLocale } from '../lib/i18n';
import { activity } from '../lib/live';
import PlanCard from './PlanCard.svelte';

const plan: PlanSummary = {
  id: 'PLAN-auth-001',
  project: 'auth-gateway',
  title: 'Migrate sessions to JWT',
  tags: ['type/feature', 'tech/jwt'],
  created: '2026-11-20',
  task_count: 2,
  done_count: 1,
  structured: true,
  warnings_count: 0,
};
const task = (id: string, name: string, status: Task['status']): Task => ({
  id,
  name,
  complexity: 3,
  model: null,
  effort: null,
  depends_on: [],
  status,
  tags: [],
  body_md: '',
});
const tasks = [
  task('TASK-001', 'Auth service', 'done'),
  task('TASK-002', 'Token Management', 'in_progress'),
];
const live: LiveTask = {
  plan_id: 'PLAN-auth-001',
  task_id: 'TASK-002',
  agents: [
    {
      agent_id: 'a1',
      role: 'main',
      model: null,
      step: { icon: '📖', text_en: 'read token.rs', text_vi: 'đọc token.rs', raw: 'Read token.rs' },
      at: 1,
    },
  ],
};

afterEach(() => {
  cleanup();
  activity.set([]);
  drawer.close();
  setLocale('en');
  location.hash = '';
});

describe('PlanCard', () => {
  it('plan_card_shows_live_nest_only_when_active', async () => {
    setLocale('en');
    const { container } = render(PlanCard, {
      plan,
      tasks,
      labels: { 'tech/jwt': 'JWT login token' },
    });
    expect(screen.getByText('📄 Plan · auth-gateway')).toBeTruthy();
    expect(screen.getByText('JWT login token')).toBeTruthy();
    expect(screen.queryByText('Feature')).toBeNull();
    expect(container.querySelector('.nest')).toBeNull();
    expect(container.querySelector('.has-live')).toBeNull();

    activity.set([live]);
    await Promise.resolve();
    expect(container.querySelector('.nest')).not.toBeNull();
    expect(container.querySelector('.has-live')).not.toBeNull();
    expect(screen.getByText('Claude is working on 1 task of this plan')).toBeTruthy();
    expect(screen.getByText('◐ TASK-002 · Token Management')).toBeTruthy();
    expect(container.querySelector('.nag')?.textContent).toMatch(
      /Coding assistant\s+📖\s+is reading token\.rs/,
    );

    // Another plan's activity does not count.
    activity.set([{ ...live, plan_id: 'PLAN-other-001' }]);
    await Promise.resolve();
    expect(container.querySelector('.nest')).toBeNull();
  });

  it('live nest speaks Vietnamese with "đang"', async () => {
    setLocale('vi');
    activity.set([live]);
    const { container } = render(PlanCard, { plan, tasks });
    expect(container.querySelector('.nag')?.textContent).toMatch(
      /Trợ lý lập trình\s+📖\s+đang đọc token\.rs/,
    );
  });

  it('clicking a live task opens its drawer instead of the plan', async () => {
    location.hash = '#/home';
    activity.set([live]);
    render(PlanCard, { plan, tasks });
    await fireEvent.click(screen.getByText('◐ TASK-002 · Token Management'));
    expect(location.hash).toBe('#/home');
    expect(get(drawer)).toEqual([{ type: 'task', plan: 'PLAN-auth-001', task: 'TASK-002' }]);
  });

  it('clicking the card opens the plan', async () => {
    location.hash = '#/home';
    const { container } = render(PlanCard, { plan, tasks });
    await fireEvent.click(container.querySelector('h3')!);
    expect(location.hash).toBe('#/plan/PLAN-auth-001/docs');
  });
});
