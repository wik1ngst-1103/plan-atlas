import { writable } from 'svelte/store';
import { api } from './api';
import type { LiveTask } from './generated/LiveTask';
import type { AgentView } from './generated/AgentView';
import type { TaskStatus } from './generated/TaskStatus';

/** Mirrors the server's `LiveEvent` (serde tag = "type"). */
export type LiveEvent =
  | { type: 'plan_changed'; plan_id: string }
  | { type: 'task_status'; plan_id: string; task_id: string; status: TaskStatus }
  | { type: 'agent_step'; plan_id: string; task_id: string; agent: AgentView }
  | { type: 'index_ready' };
export type LiveEventType = LiveEvent['type'];

const NAMES: LiveEventType[] = ['plan_changed', 'task_status', 'agent_step', 'index_ready'];
const DEBOUNCE_MS = 300;

export const connected = writable(false);
export const activity = writable<LiveTask[]>([]);

const listeners = new Map<LiveEventType, Set<(e: LiveEvent) => void>>();

/** Subscribe to one event type; returns the unsubscribe function. */
export function onEvent<T extends LiveEventType>(
  type: T,
  cb: (e: Extract<LiveEvent, { type: T }>) => void,
): () => void {
  const set = listeners.get(type) ?? new Set();
  const fn = cb as (e: LiveEvent) => void;
  set.add(fn);
  listeners.set(type, set);
  return () => set.delete(fn);
}

export async function refreshActivity() {
  try {
    activity.set(await api.get<LiveTask[]>('/api/activity'));
  } catch {
    /* keep the last known activity */
  }
}

let timer: ReturnType<typeof setTimeout> | undefined;
function refreshSoon() {
  clearTimeout(timer);
  timer = setTimeout(refreshActivity, DEBOUNCE_MS);
}

let source: EventSource | undefined;

/** Open the single SSE connection (idempotent). EventSource reconnects by itself. */
export function startLive() {
  if (source) return;
  void refreshActivity(); // don't wait for the stream to open (proxies may hold it)
  source = new EventSource('/api/stream');
  source.onopen = () => {
    connected.set(true);
    void refreshActivity();
  };
  source.onerror = () => connected.set(false);
  for (const name of NAMES) {
    source.addEventListener(name, (ev) => {
      let data: LiveEvent;
      try {
        data = JSON.parse((ev as MessageEvent<string>).data) as LiveEvent;
      } catch {
        return;
      }
      if (name === 'agent_step' || name === 'task_status') refreshSoon();
      listeners.get(name)?.forEach((cb) => cb(data));
    });
  }
}

export function stopLive() {
  source?.close();
  source = undefined;
  clearTimeout(timer);
  connected.set(false);
}
