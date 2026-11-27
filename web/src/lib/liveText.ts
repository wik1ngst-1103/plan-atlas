import type { AgentView } from './generated/AgentView';
import type { LiveTask } from './generated/LiveTask';
import type { Step } from './generated/Step';
import type { Locale } from './i18n';

type T = (key: string, params?: Record<string, string | number>) => string;

const ROLES = ['main', 'Explore', 'general-purpose', 'Plan', 'implementer'];

/** Plain-language name of an agent role ("Coding assistant"), "Assistant" when unknown. */
export const roleLabel = (t: T, role: string) =>
  ROLES.includes(role) ? t(`role.${role}`) : t('role.other');

const IRREGULAR: Record<string, string> = {
  run: 'running',
  get: 'getting',
  set: 'setting',
  put: 'putting',
  stop: 'stopping',
};

/** English -ing form of the first word of a verb phrase: "read x" → "reading x". */
export function gerund(phrase: string): string {
  const [verb, ...rest] = phrase.split(' ');
  const w = verb.toLowerCase();
  let ing: string;
  if (IRREGULAR[w]) ing = IRREGULAR[w];
  else if (/[^e]e$/.test(w)) ing = w.slice(0, -1) + 'ing';
  else ing = w + 'ing';
  return [ing, ...rest].join(' ');
}

const IRREGULAR_PAST: Record<string, string> = {
  read: 'read',
  run: 'ran',
  get: 'got',
  set: 'set',
  put: 'put',
  find: 'found',
  write: 'wrote',
  make: 'made',
  stop: 'stopped',
};

/** English past form of the first word of a verb phrase: "run tests" → "ran tests". */
export function pastTense(phrase: string): string {
  const [verb, ...rest] = phrase.split(' ');
  const w = verb.toLowerCase();
  let past: string;
  if (IRREGULAR_PAST[w]) past = IRREGULAR_PAST[w];
  else if (w.endsWith('e')) past = w + 'd';
  else if (/[^aeiou]y$/.test(w)) past = w.slice(0, -1) + 'ied';
  else past = w + 'ed';
  return [past, ...rest].join(' ');
}

/** The step as a past-tense sentence part for logs: "ran tests" / "đã chạy kiểm thử". */
export function pastStep(t: T, l: Locale, step: Step): string {
  return t('live.done', { what: l === 'vi' ? step.text_vi : pastTense(step.text_en) });
}

/** The step as a current-tense sentence part: "is reading token.rs" / "đang đọc token.rs". */
export function presentStep(t: T, l: Locale, step: Step): string {
  return t('live.doing', { what: l === 'vi' ? step.text_vi : gerund(step.text_en) });
}

/** "Coding assistant is reading token.rs" for the task's most recent agent, or "". */
export function taskLiveText(t: T, l: Locale, live: LiveTask | undefined): string {
  const a: AgentView | undefined = live
    ? [...live.agents].sort((x, y) => y.at - x.at)[0]
    : undefined;
  return a ? `${roleLabel(t, a.role)} ${presentStep(t, l, a.step)}` : '';
}
