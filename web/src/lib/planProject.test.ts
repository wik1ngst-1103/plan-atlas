import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearPlanProjectCache, planProject } from './planProject';

afterEach(() => {
  vi.unstubAllGlobals();
  clearPlanProjectCache();
});

describe('planProject', () => {
  it('uses the plan project field (hyphenated) and caches it', async () => {
    const f = vi.fn().mockResolvedValue(new Response('{"project":"auth-gateway"}'));
    vi.stubGlobal('fetch', f);
    expect(await planProject('hand-edited-id')).toBe('auth-gateway');
    expect(await planProject('hand-edited-id')).toBe('auth-gateway');
    expect(f).toHaveBeenCalledTimes(1);
  });

  it('returns null when unknown', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response('{"error":"x"}', { status: 404 })),
    );
    expect(await planProject('nope')).toBeNull();
  });
});
