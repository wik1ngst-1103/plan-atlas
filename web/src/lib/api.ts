import { currentLocale, type Locale } from './i18n';

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

const LOCALIZED = [
  /^\/api\/topics$/,
  /^\/api\/activity$/,
  /^\/api\/search$/,
  /^\/api\/plans\/[^/]+\/tasks\/[^/]+\/log$/,
];

/** Append `locale` to the endpoints whose answers are worded per language. */
export function withLocale(path: string, locale: Locale): string {
  const [base, query] = path.split('?');
  if (!LOCALIZED.some((re) => re.test(base))) return path;
  return `${path}${query === undefined ? '?' : '&'}locale=${locale}`;
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = { 'X-Plan-Atlas': '1' };
  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    init.body = JSON.stringify(body);
  }
  const res = await fetch(withLocale(path, currentLocale()), init);
  const text = await res.text();
  let data: unknown = undefined;
  try {
    data = text ? JSON.parse(text) : undefined;
  } catch {
    /* non-JSON body */
  }
  if (!res.ok) {
    const msg = (data as { error?: string } | undefined)?.error ?? (text || res.statusText);
    throw new ApiError(res.status, msg);
  }
  return data as T;
}

export const api = {
  get: <T>(path: string) => request<T>('GET', path),
  patch: <T>(path: string, body: unknown) => request<T>('PATCH', path, body),
};
