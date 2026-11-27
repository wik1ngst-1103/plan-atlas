import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, describe, expect, it } from 'vitest';
import { drawer } from '../lib/drawer';
import type { Task } from '../lib/generated/Task';
import { setLocale } from '../lib/i18n';
import TaskRow from './TaskRow.svelte';

const task: Task = {
  id: 'TASK-002',
  name: 'Token Management',
  complexity: 3,
  model: null,
  effort: null,
  depends_on: [],
  status: 'in_progress',
  tags: [],
  body_md: '',
};

afterEach(() => {
  cleanup();
  drawer.close();
  location.hash = '';
});

describe('TaskRow', () => {
  it('task_row_click_opens_drawer_not_navigation', async () => {
    setLocale('en');
    location.hash = '#/plan/PLAN-auth-001/docs';
    render(TaskRow, { planId: 'PLAN-auth-001', task, rowId: 'task-002-token-management' });
    expect(screen.getByText(/Medium \(3\/5\)/)).toBeTruthy();
    expect(screen.getByText(/Claude uses Sonnet/)).toBeTruthy();

    await fireEvent.click(screen.getByText('TASK-002 · Token Management'));
    expect(location.hash).toBe('#/plan/PLAN-auth-001/docs');
    expect(get(drawer)).toHaveLength(1);
    expect(get(drawer)[0]).toEqual({ type: 'task', plan: 'PLAN-auth-001', task: 'TASK-002' });
  });
});
