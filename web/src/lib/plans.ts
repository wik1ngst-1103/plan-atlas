import type { LiveTask } from './generated/LiveTask';
import type { Locale } from './i18n';
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
