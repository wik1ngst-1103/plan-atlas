import { derived, get, writable } from 'svelte/store';
import { api } from './api';
import { locale, type Locale } from './i18n';
import { onEvent } from './live';

/** One entry of `GET /api/topics` (label/desc already in the requested locale). */
export type Topic = {
  tag: string;
  group: string;
  label: string;
  desc: string;
  plans: number;
  tasks: number;
};

export const topics = writable<Topic[]>([]);

/** tag → localized plain label (unknown tags are left out; chips then show the bare name). */
export const topicLabels = derived(topics, (ts) =>
  Object.fromEntries(ts.filter((t) => t.label !== t.tag).map((t) => [t.tag, t.label])),
);

let loadedFor: Locale | null = null;
let watching = false;

export async function refreshTopics() {
  const l = get(locale);
  try {
    const ts = await api.get<Topic[]>('/api/topics');
    if (get(locale) === l) {
      topics.set(ts);
      loadedFor = l;
    }
  } catch {
    /* keep the last known topics; chips fall back to the tag name */
  }
}

/** Load topics once per locale; reloads by itself on a locale change or a finished re-index. */
export function ensureTopics() {
  if (!watching) {
    watching = true;
    locale.subscribe((l) => {
      if (loadedFor !== null && loadedFor !== l) void refreshTopics();
    });
    onEvent('index_ready', () => void refreshTopics());
  }
  if (loadedFor !== get(locale)) void refreshTopics();
}
