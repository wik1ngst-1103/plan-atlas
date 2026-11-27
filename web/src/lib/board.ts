import { writable } from 'svelte/store';
import type { TaskRow as TaskItem } from './generated/TaskRow';
import type { TaskStatus } from './generated/TaskStatus';
import { MODEL_NAME, taskModel } from './plans';

export type BoardGroup = 'status' | 'model' | 'domain';
export type BoardFilter = { project: string; tag: string; group: BoardGroup };
export type Column = { key: string; label: string };

export const ALL = '';
export const NONE = '_none';
export const STATUSES: TaskStatus[] = ['todo', 'in_progress', 'review', 'done'];
export const STATUS_ICON: Record<TaskStatus, string> = {
  todo: '○',
  in_progress: '◐',
  review: '◑',
  done: '●',
};

export const defaultFilter = (): BoardFilter => ({ project: ALL, tag: ALL, group: 'status' });

/** Board filters; a module store so coming back to the board keeps them. */
export const boardFilter = writable<BoardFilter>(defaultFilter());

/** Topics of a task, its own and its plan's. */
export const itemTags = (i: TaskItem) => [...i.task.tags, ...i.plan_tags];

export const isFiltered = (f: BoardFilter) => f.project !== ALL || f.tag !== ALL;

export function filterItems(items: TaskItem[], f: BoardFilter): TaskItem[] {
  return items.filter(
    (i) =>
      (f.project === ALL || i.project === f.project) &&
      (f.tag === ALL || itemTags(i).includes(f.tag)),
  );
}

/** Column an item falls into for a grouping. */
export function columnKey(i: TaskItem, group: BoardGroup): string {
  if (group === 'status') return i.task.status;
  if (group === 'model') return taskModel(i.task) ?? NONE;
  return itemTags(i).find((t) => t.startsWith('domain/')) ?? NONE;
}

type T = (key: string, params?: Record<string, string | number>) => string;

/**
 * Columns for a grouping. Domain columns come from every item (so they stay put while
 * filtering), plus "Unclassified"; the model "not set" column only when some task needs it.
 */
export function columnsFor(
  group: BoardGroup,
  all: TaskItem[],
  t: T,
  labels: Record<string, string>,
): Column[] {
  if (group === 'status')
    return STATUSES.map((s) => ({ key: s, label: `${STATUS_ICON[s]} ${t(`status.${s}`)}` }));
  if (group === 'model') {
    const cols: Column[] = (['haiku', 'sonnet', 'opus'] as const).map((m) => ({
      key: m,
      label: t('diff.uses', { model: MODEL_NAME[m] }),
    }));
    if (all.some((i) => !taskModel(i.task))) cols.push({ key: NONE, label: t('board.noModel') });
    return cols;
  }
  const domains = [...new Set(all.flatMap(itemTags).filter((x) => x.startsWith('domain/')))].sort();
  return [
    ...domains.map((d) => ({ key: d, label: labels[d] ?? d.slice('domain/'.length) })),
    { key: NONE, label: t('group.none') },
  ];
}

/** Distinct topics used by the items, grouped type → domain → tech (others last). */
export function topicGroups(all: TaskItem[], extra: string): { group: string; tags: string[] }[] {
  const tags = new Set(all.flatMap(itemTags));
  if (extra) tags.add(extra);
  const sorted = [...tags].sort();
  const known = ['type', 'domain', 'tech'];
  const groups = known.map((g) => ({
    group: g,
    tags: sorted.filter((x) => x.startsWith(`${g}/`)),
  }));
  const rest = sorted.filter((x) => !known.includes(x.split('/')[0]));
  if (rest.length) groups.push({ group: 'other', tags: rest });
  return groups.filter((g) => g.tags.length);
}

/** Same list with one task's status changed (optimistic board move). */
export const withStatus = (items: TaskItem[], plan: string, task: string, status: TaskStatus) =>
  items.map((i) =>
    i.plan_id === plan && i.task.id === task ? { ...i, task: { ...i.task, status } } : i,
  );
