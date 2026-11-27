import type { LiveTask } from './generated/LiveTask';
import type { Locale } from './i18n';
import type { Effort } from './generated/Effort';
import type { Model } from './generated/Model';
import type { PlanSummary } from './generated/PlanSummary';
import type { Section } from './generated/Section';
import type { Task } from './generated/Task';

export type PlanStatus = 'todo' | 'in_progress' | 'done';

/** Overall plan state from its task counts (null when the plan has no task list). */
export function planStatus(done: number, total: number, anyStarted: boolean): PlanStatus | null {
  if (total === 0) return null;
  if (done === total) return 'done';
  return done > 0 || anyStarted ? 'in_progress' : 'todo';
}

const DEFAULT_MODEL: Record<number, Model> = {
  1: 'haiku',
  2: 'sonnet',
  3: 'sonnet',
  4: 'opus',
  5: 'opus',
};

/** The model Claude uses for a task: explicit, else from complexity, else null. */
export const taskModel = (task: Task): Model | null =>
  task.model ?? (task.complexity ? (DEFAULT_MODEL[task.complexity] ?? null) : null);

const DEFAULT_EFFORT: Record<number, Effort> = {
  1: 'low',
  2: 'low',
  3: 'medium',
  4: 'medium',
  5: 'high',
};

/** How hard Claude thinks about a task: explicit, else from complexity, else null. */
export const taskEffort = (task: Task): Effort | null =>
  task.effort ?? (task.complexity ? (DEFAULT_EFFORT[task.complexity] ?? null) : null);

export const MODEL_NAME: Record<Model, string> = { haiku: 'Haiku', sonnet: 'Sonnet', opus: 'Opus' };
export const MODEL_COLOR: Record<Model, string> = {
  haiku: '#0ca678',
  sonnet: '#4263eb',
  opus: '#ae3ec9',
};

/** Same tree with anchors made unique (slug collisions get -2, -3…). */
export function uniqueAnchors(sections: Section[]): Section[] {
  const seen = new Map<string, number>();
  const walk = (ss: Section[]): Section[] =>
    ss.map((s) => {
      const base = s.anchor || 'section';
      const n = (seen.get(base) ?? 0) + 1;
      seen.set(base, n);
      return { ...s, anchor: n === 1 ? base : `${base}-${n}`, children: walk(s.children) };
    });
  return walk(sections);
}

export const isTaskSection = (s: Section) => s.title.trim().startsWith('04.');
export const taskIdOf = (s: Section) => /^(TASK-\d{3})\b/.exec(s.title.trim())?.[1] ?? null;

/** "20 Nov 2026" / "20 thg 11, 2026" from "2026-11-20" (no timezone shift). */
export function formatDate(iso: string, l: Locale): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
  if (!m) return iso;
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  return d.toLocaleDateString(l === 'vi' ? 'vi-VN' : 'en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
}

/** Plans with Claude working on them first (more active tasks first); otherwise keep order. */
export function liveFirst(plans: PlanSummary[], activity: LiveTask[]): PlanSummary[] {
  const n = (id: string) => activity.filter((a) => a.plan_id === id && a.agents.length).length;
  return plans
    .map((p, i) => ({ p, i, n: n(p.id) }))
    .sort((a, b) => b.n - a.n || a.i - b.i)
    .map((x) => x.p);
}

/** Distinct tasks Claude is working on. */
export const runningCount = (activity: LiveTask[]) =>
  new Set(activity.filter((a) => a.agents.length).map((a) => `${a.plan_id}/${a.task_id}`)).size;

/** "01. Overview" → { n: '01', name: 'Overview' }: one of the five standard plan sections. */
export function standardSection(title: string): { n: string; name: string } | null {
  const m = /^(0[1-5])\.\s*(.*)$/.exec(title.trim());
  return m ? { n: m[1], name: m[2] || title.trim() } : null;
}

/**
 * Headings that name a file, as imported plans often have:
 * "New file: server/src/a.ts" → { label: 'New file', path: 'server/src/a.ts', note: '' },
 * "src/b.ts (wire it up)" → { label: '', path: 'src/b.ts', note: '(wire it up)' }.
 * The path must come first (after an optional "Label:") and look like one: a slash, or a
 * name with an extension.
 */
export function pathHeading(title: string): { label: string; path: string; note: string } | null {
  const m =
    /^(?:([^:/`]{1,24}):\s+)?`?((?:[\w.@~-]+\/)+[\w.@~-]+\/?|[\w@~-][\w.@~-]*\.[a-z][a-z0-9]{0,5})`?(?:\s+(.+))?$/i.exec(
      title.trim(),
    );
  if (!m) return null;
  // A bare "Node.js" or "Vue.js vs React" is prose; a slash-less name needs a label or to stand
  // alone in lower case ("package.json").
  if (!m[2].includes('/') && !m[1] && (m[3] || !/^[a-z_.@~-]/.test(m[2]))) return null;
  return { label: m[1]?.trim() ?? '', path: m[2], note: m[3]?.trim() ?? '' };
}

/** Minutes to read the plan at ~220 words a minute (code counts, it gets skimmed too). */
export function readingMinutes(sections: Section[]): number {
  let words = 0;
  const walk = (ss: Section[]) => {
    for (const s of ss) {
      words += `${s.title} ${s.body_md}`.split(/\s+/).filter((w) => /\w/.test(w)).length;
      walk(s.children);
    }
  };
  walk(sections);
  return Math.max(1, Math.round(words / 220));
}
