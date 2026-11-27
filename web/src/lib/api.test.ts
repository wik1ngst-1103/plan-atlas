import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, api, withLocale } from './api';

afterEach(() => vi.unstubAllGlobals());

describe('api', () => {
  it('adds locale only to locale-aware endpoints', () => {
    expect(withLocale('/api/topics', 'vi')).toBe('/api/topics?locale=vi');
    expect(withLocale('/api/search?q=a', 'vi')).toBe('/api/search?q=a&locale=vi');
    expect(withLocale('/api/activity', 'en')).toBe('/api/activity?locale=en');
    expect(withLocale('/api/plans/P-1/tasks/TASK-001/log?limit=5', 'vi')).toBe(
      '/api/plans/P-1/tasks/TASK-001/log?limit=5&locale=vi',
    );
    expect(withLocale('/api/projects', 'vi')).toBe('/api/projects');
    expect(withLocale('/api/plans/P-1', 'vi')).toBe('/api/plans/P-1');
  });

  it('patch sends the CSRF header and JSON body', async () => {
    const f = vi.fn().mockResolvedValue(new Response('{"ok":true}', { status: 200 }));
    vi.stubGlobal('fetch', f);
    await api.patch('/api/plans/P/tasks/T', { status: 'done' });
    const [url, init] = f.mock.calls[0];
    expect(url).toBe('/api/plans/P/tasks/T');
    expect(init.method).toBe('PATCH');
    expect(init.headers['X-Plan-Atlas']).toBe('1');
    expect(init.headers['Content-Type']).toBe('application/json');
    expect(init.body).toBe('{"status":"done"}');
  });

  it('throws ApiError parsed from {error}', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response('{"error":"nope"}', { status: 404 })),
    );
    await expect(api.get('/api/x')).rejects.toEqual(new ApiError(404, 'nope'));
  });
});
