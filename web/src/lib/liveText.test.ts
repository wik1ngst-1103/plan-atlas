import { describe, expect, it } from 'vitest';
import { translate } from './i18n';
import { gerund, presentStep, roleLabel } from './liveText';

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
