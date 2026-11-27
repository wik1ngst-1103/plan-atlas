import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { get } from 'svelte/store';
import { afterEach, describe, expect, it } from 'vitest';
import { drawer } from '../lib/drawer';
import type { Section } from '../lib/generated/Section';
import type { Task } from '../lib/generated/Task';
import { setLocale } from '../lib/i18n';
import DocView from './DocView.svelte';

const s = (title: string, anchor: string, body_md = '', children: Section[] = []): Section => ({
  level: 2,
  title,
  anchor,
  body_md,
  children,
});
const task: Task = {
  id: 'TASK-001',
  name: 'Auth service',
  complexity: 2,
  model: null,
  effort: null,
  depends_on: [],
  status: 'todo',
  tags: [],
  body_md: '',
};

afterEach(() => {
  cleanup();
  drawer.close();
  location.hash = '';
});

describe('DocView', () => {
  it('numbers standard sections and keeps heading ids', () => {
    setLocale('vi');
    const { container } = render(DocView, {
      planId: 'P',
      sections: [s('01. Overview', '01-overview', 'Hello')],
      tasks: [],
    });
    const h2 = container.querySelector('h2[id="01-overview"]')!;
    expect(h2.querySelector('.marker')?.textContent).toBe('01');
    expect(h2.textContent).toContain('Overview');
    expect(h2.querySelector('small')?.textContent).toBe('Tổng quan');
    setLocale('en');
  });

  it('renders file-path headings as code with the label apart', () => {
    const { container } = render(DocView, {
      planId: 'P',
      sections: [
        s('Phase 1', 'phase-1', '', [{ ...s('New file: server/src/a/b.ts', 'nf', 'x'), level: 3 }]),
      ],
      tasks: [],
    });
    const h3 = container.querySelector('h3#nf')!;
    expect(h3.classList.contains('path')).toBe(true);
    expect(h3.querySelector('.plabel')?.textContent).toBe('New file');
    expect(h3.querySelector('code .dir')?.textContent).toBe('server/src/a/');
    expect(h3.querySelector('code .base')?.textContent).toBe('b.ts');
  });

  it('keeps sanitizing and task rows still open the drawer', async () => {
    location.hash = '#/plan/P/docs';
    const { container } = render(DocView, {
      planId: 'P',
      sections: [s('04. Task Breakdown', '04', '<img src=x onerror="alert(1)"><script>x</script>')],
      tasks: [task],
    });
    expect(container.innerHTML).not.toContain('onerror');
    expect(container.innerHTML).not.toContain('<script');
    await fireEvent.click(container.querySelector('.trow')!);
    expect(location.hash).toBe('#/plan/P/docs');
    expect(get(drawer)[0]).toEqual({ type: 'task', plan: 'P', task: 'TASK-001' });
  });
});
