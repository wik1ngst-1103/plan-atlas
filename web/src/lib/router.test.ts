import { describe, expect, it } from 'vitest';
import { formatRoute, parseRoute } from './router';

describe('router', () => {
  it('router_parses_all_routes', () => {
    expect(parseRoute('#/home')).toEqual({ page: 'home' });
    expect(parseRoute('#/project/auth-gateway')).toEqual({
      page: 'project',
      project: 'auth-gateway',
    });
    expect(parseRoute('#/plan/PLAN-auth-001/docs')).toEqual({
      page: 'plan',
      plan: 'PLAN-auth-001',
      tab: 'docs',
    });
    expect(parseRoute('#/plan/PLAN-auth-001/tasks')).toEqual({
      page: 'plan',
      plan: 'PLAN-auth-001',
      tab: 'tasks',
    });
    expect(parseRoute('#/plan/PLAN-auth-001')).toEqual({
      page: 'plan',
      plan: 'PLAN-auth-001',
      tab: 'docs',
    });
    expect(parseRoute('#/board')).toEqual({ page: 'board' });
    expect(parseRoute('#/map')).toEqual({ page: 'map' });
  });

  it('falls back to home and decodes segments', () => {
    expect(parseRoute('')).toEqual({ page: 'home' });
    expect(parseRoute('#/nope')).toEqual({ page: 'home' });
    expect(parseRoute('#/project/a%20b')).toEqual({ page: 'project', project: 'a b' });
  });

  it('round-trips through formatRoute', () => {
    const r = { page: 'plan', plan: 'PLAN-x-001', tab: 'tasks' } as const;
    expect(parseRoute(formatRoute(r))).toEqual(r);
  });
});
