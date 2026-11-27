import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { activity, startLive, stopLive } from './live';

class FakeSource {
  static last: FakeSource | undefined;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  handlers = new Map<string, (ev: MessageEvent<string>) => void>();
  constructor(public url: string) {
    FakeSource.last = this;
  }
  addEventListener(name: string, fn: (ev: MessageEvent<string>) => void) {
    this.handlers.set(name, fn);
  }
  emit(name: string, data: unknown) {
    this.handlers.get(name)?.(new MessageEvent(name, { data: JSON.stringify(data) }));
  }
  close() {}
}

const live = [
  {
    plan_id: 'PLAN-a-001',
    task_id: 'TASK-001',
    agents: [{ agent_id: 'main', role: 'main', model: null, step: {}, at: 1 }],
  },
];

describe('live activity', () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  beforeEach(() => {
    vi.useFakeTimers();
    fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(live)));
    vi.stubGlobal('fetch', fetchMock);
    vi.stubGlobal('EventSource', FakeSource);
  });
  afterEach(() => {
    stopLive();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('refetches /api/activity on activity_changed', async () => {
    startLive();
    await vi.runAllTimersAsync();
    expect(get(activity)).toEqual(live);
    const calls = fetchMock.mock.calls.length;

    fetchMock.mockResolvedValue(new Response('[]'));
    FakeSource.last!.emit('activity_changed', { type: 'activity_changed' });
    await vi.runAllTimersAsync();
    expect(fetchMock.mock.calls.length).toBe(calls + 1);
    expect(String(fetchMock.mock.calls.at(-1)![0])).toMatch(/^\/api\/activity/);
    expect(get(activity)).toEqual([]);
  });

  it('does not refetch activity on plan_changed', async () => {
    startLive();
    await vi.runAllTimersAsync();
    const calls = fetchMock.mock.calls.length;
    FakeSource.last!.emit('plan_changed', { type: 'plan_changed', plan_id: 'PLAN-a-001' });
    await vi.runAllTimersAsync();
    expect(fetchMock.mock.calls.length).toBe(calls);
  });
});
