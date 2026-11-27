import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { colorFor, projectColor, setKnownProjects } from './projectColor';

describe('project colours', () => {
  it('are distinct for up to six known projects, by sorted index', () => {
    // These two collide under the old character-sum hash.
    const names = ['ab', 'ba', 'web', 'api', 'docs', 'infra'];
    const sorted = [...names].sort();
    const colors = sorted.map((n) => colorFor(n, sorted));
    expect(new Set(colors).size).toBe(6);
  });

  it('do not depend on the order projects arrive in', () => {
    setKnownProjects(['web', 'api', 'docs']);
    const a = get(projectColor)('web');
    setKnownProjects(['docs', 'web', 'api', 'web']);
    expect(get(projectColor)('web')).toBe(a);
  });

  it('fall back to a stable colour for unknown names', () => {
    setKnownProjects(['api']);
    const c = get(projectColor)('zzz');
    expect(c).toMatch(/^#[0-9a-f]{6}$/);
    expect(get(projectColor)('zzz')).toBe(c);
  });
});
