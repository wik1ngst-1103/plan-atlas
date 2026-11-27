import { describe, expect, it } from 'vitest';
import type { Section } from './generated/Section';
import { renderMarkdown } from './markdown';
import {
  formatDate,
  pathHeading,
  planStatus,
  readingMinutes,
  standardSection,
  uniqueAnchors,
} from './plans';

const sec = (anchor: string, children: Section[] = []): Section => ({
  level: 2,
  title: anchor,
  anchor,
  body_md: '',
  children,
});

describe('plans helpers', () => {
  it('derives the plan status from its tasks', () => {
    expect(planStatus(0, 0, false)).toBeNull();
    expect(planStatus(0, 3, false)).toBe('todo');
    expect(planStatus(0, 3, true)).toBe('in_progress');
    expect(planStatus(1, 3, false)).toBe('in_progress');
    expect(planStatus(3, 3, false)).toBe('done');
  });

  it('makes duplicate heading anchors unique', () => {
    const out = uniqueAnchors([sec('notes', [sec('notes')]), sec('notes')]);
    expect([out[0].anchor, out[0].children[0].anchor, out[1].anchor]).toEqual([
      'notes',
      'notes-2',
      'notes-3',
    ]);
  });

  it('formats dates without a timezone shift', () => {
    expect(formatDate('2026-11-20', 'en')).toContain('20');
    expect(formatDate('2026-11-20', 'en')).toContain('2026');
    expect(formatDate('not a date', 'en')).toBe('not a date');
  });

  it('renders markdown and strips scripts', () => {
    const html = renderMarkdown(
      '**bold** <script>alert(1)</script><img src=x onerror=alert(1)> [x](https://e.com)',
    );
    expect(html).toContain('<strong>bold</strong>');
    expect(html).not.toContain('<script');
    expect(html).not.toContain('onerror');
    expect(html).toContain('target="_blank"');
  });
});

describe('reading helpers', () => {
  it('standardSection reads the five numbered sections', () => {
    expect(standardSection('01. Overview')).toEqual({ n: '01', name: 'Overview' });
    expect(standardSection(' 04. Task Breakdown ')).toEqual({ n: '04', name: 'Task Breakdown' });
    expect(standardSection('06. Extra')).toBeNull();
    expect(standardSection('Context')).toBeNull();
  });

  it('pathHeading recognises file headings and leaves prose alone', () => {
    expect(pathHeading('New file: server/src/agents/proof/proof.compiler.ts')).toEqual({
      label: 'New file',
      path: 'server/src/agents/proof/proof.compiler.ts',
      note: '',
    });
    expect(pathHeading('server/src/agents/scene.agent.ts (consume the proof block)')).toEqual({
      label: '',
      path: 'server/src/agents/scene.agent.ts',
      note: '(consume the proof block)',
    });
    expect(pathHeading('`web/src/app.css`')?.path).toBe('web/src/app.css');
    expect(pathHeading('package.json')?.path).toBe('package.json');
    expect(pathHeading('Modify: lib.rs')?.label).toBe('Modify');
    for (const prose of [
      'Wiring (creative-pipeline.service.ts)',
      'Tests (plain JS node:test, run against dist/)',
      'Node.js',
      'Vue.js vs React',
      'Phase 1 — Proof-First claim→proof layer',
      '01. Overview',
    ]) {
      expect(pathHeading(prose), prose).toBeNull();
    }
  });

  it('readingMinutes counts words across the tree, at least one minute', () => {
    const words = (n: number) => Array.from({ length: n }, () => 'word').join(' ');
    const s = (body: string, children: Section[] = []): Section => ({
      level: 2,
      title: 'T',
      anchor: 't',
      body_md: body,
      children,
    });
    expect(readingMinutes([s('hi')])).toBe(1);
    expect(readingMinutes([s(words(600), [s(words(500))])])).toBe(5);
  });
});
