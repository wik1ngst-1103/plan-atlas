import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import en from '../locales/en.json';
import vi from '../locales/vi.json';

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    if (statSync(p).isDirectory()) return n === 'generated' ? [] : files(p);
    return /\.(svelte|ts)$/.test(n) && !n.endsWith('.test.ts') ? [p] : [];
  });
}

describe('locale keys', () => {
  it('all_used_keys_exist', () => {
    const used = new Set<string>();
    for (const f of files(join(import.meta.dirname, '..'))) {
      for (const m of readFileSync(f, 'utf8').matchAll(/\bt\(\s*['"]([\w.]+)['"]/g)) used.add(m[1]);
    }
    expect(used.size).toBeGreaterThan(10);
    const missing = [...used].filter((k) => !(k in en) || !(k in vi));
    expect(missing).toEqual([]);

    // Dynamic keys built in components.
    const dynamic = [
      ...['task', 'topic', 'plan', 'activity'].map((x) => `drawer.${x}`),
      ...['todo', 'in_progress', 'review', 'done'].map((x) => `status.${x}`),
    ];
    for (const k of dynamic) {
      expect(k in en, `en ${k}`).toBe(true);
      expect(k in vi, `vi ${k}`).toBe(true);
    }
  });
});
