import { describe, expect, it } from 'vitest';
import en from '../locales/en.json';
import vi from '../locales/vi.json';
import { resolveInitialLocale, translate } from './i18n';

describe('i18n', () => {
  it('i18n_falls_back_to_en', () => {
    const dicts = { en: { a: 'A {n}', only: 'EN only' }, vi: { a: 'Á {n}' } };
    expect(translate('vi', 'a', { n: 2 }, dicts)).toBe('Á 2');
    expect(translate('vi', 'only', undefined, dicts)).toBe('EN only');
    expect(translate('vi', 'missing', undefined, dicts)).toBe('missing');
  });

  it('locales_have_identical_keys', () => {
    expect(Object.keys(vi).sort()).toEqual(Object.keys(en).sort());
  });

  it('initial_locale_from_navigator', () => {
    expect(resolveInitialLocale('vi-VN', null)).toBe('vi');
    expect(resolveInitialLocale('fr', null)).toBe('en');
    expect(resolveInitialLocale('en-US', 'vi')).toBe('vi');
    expect(resolveInitialLocale('vi-VN', 'en')).toBe('en');
    expect(resolveInitialLocale('vi', 'garbage')).toBe('vi');
  });
});
