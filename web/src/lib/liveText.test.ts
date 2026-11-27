import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { translate } from './i18n';
import { gerund, pastStep, pastTense, presentStep, roleLabel } from './liveText';

const tEn = (k: string, p?: Record<string, string | number>) => translate('en', k, p);
const tVi = (k: string, p?: Record<string, string | number>) => translate('vi', k, p);

describe('liveText', () => {
  it('turns the server verb phrase into an -ing form', () => {
    expect(gerund('read token.rs')).toBe('reading token.rs');
    expect(gerund('edit a file')).toBe('editing a file');
    expect(gerund('run tests')).toBe('running tests');
    expect(gerund('use tool X')).toBe('using tool X');
    expect(gerund('update the plan')).toBe('updating the plan');
    expect(gerund('search the code')).toBe('searching the code');
  });

  it('adds tense per locale', () => {
    const step = {
      icon: '🧪',
      text_en: 'run tests',
      text_vi: 'chạy kiểm thử',
      raw: 'Bash npm test',
    };
    expect(presentStep(tEn, 'en', step)).toBe('is running tests');
    expect(presentStep(tVi, 'vi', step)).toBe('đang chạy kiểm thử');
  });

  it('names roles in plain words with a fallback', () => {
    expect(roleLabel(tEn, 'Explore')).toBe('Research assistant');
    expect(roleLabel(tEn, 'general-purpose')).toBe('General assistant');
    expect(roleLabel(tVi, 'main')).toBe('Trợ lý lập trình');
    expect(roleLabel(tEn, 'my-custom-agent')).toBe('Assistant');
  });
});

/** First words of every English step phrase in humanize.rs (ASCII-only literals like "read a file"). */
function serverVerbs(): string[] {
  const src = readFileSync(join(import.meta.dirname, '../../../src/events/humanize.rs'), 'utf8');
  const verbs = [...src.matchAll(/"([a-z]+) [ -~]*?"/g)].map((m) => m[1]);
  return [...new Set(verbs)].sort();
}

describe('liveText past tense', () => {
  // Every verb the server emits, with its -ing and past forms.
  const FORMS: Record<string, [string, string]> = {
    check: ['checking', 'checked'],
    edit: ['editing', 'edited'],
    hand: ['handing', 'handed'],
    install: ['installing', 'installed'],
    look: ['looking', 'looked'],
    read: ['reading', 'read'],
    run: ['running', 'ran'],
    search: ['searching', 'searched'],
    update: ['updating', 'updated'],
    use: ['using', 'used'],
  };

  it('knows every verb humanize.rs emits', () => {
    expect(serverVerbs()).toEqual(Object.keys(FORMS).sort());
  });

  it('turns each server verb into -ing and past forms', () => {
    for (const [verb, [ing, past]] of Object.entries(FORMS)) {
      expect(gerund(`${verb} x`)).toBe(`${ing} x`);
      expect(pastTense(`${verb} x`)).toBe(`${past} x`);
    }
  });

  it('handles regular endings', () => {
    expect(pastTense('copy files')).toBe('copied files');
    expect(pastTense('play music')).toBe('played music');
    expect(pastTense('stop the server')).toBe('stopped the server');
  });

  it('adds past tense per locale', () => {
    const step = {
      icon: '🔍',
      text_en: 'search the code',
      text_vi: 'tìm kiếm trong code',
      raw: 'Grep x',
    };
    expect(pastStep(tEn, 'en', step)).toBe('searched the code');
    expect(pastStep(tVi, 'vi', step)).toBe('đã tìm kiếm trong code');
  });
});
