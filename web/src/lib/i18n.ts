import { derived, get, writable } from 'svelte/store';
import en from '../locales/en.json';
import vi from '../locales/vi.json';

export type Locale = 'en' | 'vi';
export type Dict = Record<string, string>;
export type Dicts = Record<Locale, Dict>;
export type Params = Record<string, string | number>;

const STORAGE_KEY = 'plan-atlas.locale';
const DICTS: Dicts = { en, vi };

const isLocale = (v: unknown): v is Locale => v === 'en' || v === 'vi';

/** Stored choice wins; otherwise `vi*` navigator language → vi, anything else → en. */
export function resolveInitialLocale(
  navigatorLanguage: string | undefined,
  stored: string | null,
): Locale {
  if (isLocale(stored)) return stored;
  return (navigatorLanguage ?? '').toLowerCase().startsWith('vi') ? 'vi' : 'en';
}

/** Look up `key` in `locale`, falling back to en, then to the key itself; interpolate `{name}`. */
export function translate(
  locale: Locale,
  key: string,
  params?: Params,
  dicts: Dicts = DICTS,
): string {
  const raw = dicts[locale]?.[key] ?? dicts.en?.[key] ?? key;
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (m, name: string) =>
    name in params ? String(params[name]) : m,
  );
}

function readStored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function applyLang(l: Locale) {
  if (typeof document !== 'undefined') document.documentElement.lang = l;
}

const initial = resolveInitialLocale(
  typeof navigator === 'undefined' ? undefined : navigator.language,
  readStored(),
);
applyLang(initial);

const store = writable<Locale>(initial);

/** Current locale (read-only store; change with `setLocale`). */
export const locale = { subscribe: store.subscribe };

export function setLocale(l: Locale) {
  store.set(l);
  applyLang(l);
  try {
    localStorage.setItem(STORAGE_KEY, l);
  } catch {
    /* storage unavailable: keep the choice for this session only */
  }
}

/** Reactive translator: `$t('nav.board')`, `$t('live.working', { n: 2 })`. */
export const t = derived(store, (l) => (key: string, params?: Params) => translate(l, key, params));

export const currentLocale = (): Locale => get(store);
