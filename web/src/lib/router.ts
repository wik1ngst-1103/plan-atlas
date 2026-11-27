import { readable } from 'svelte/store';

export type PlanTab = 'docs' | 'tasks';
export type Route =
  | { page: 'home' }
  | { page: 'project'; project: string }
  | { page: 'plan'; plan: string; tab: PlanTab }
  | { page: 'board' }
  | { page: 'map' };

const dec = (s: string) => {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
};

export function parseRoute(hash: string): Route {
  const [page, a, b] = hash.replace(/^#\/?/, '').split('/').map(dec);
  switch (page) {
    case 'project':
      return a ? { page: 'project', project: a } : { page: 'home' };
    case 'plan':
      return a
        ? { page: 'plan', plan: a, tab: b === 'tasks' ? 'tasks' : 'docs' }
        : { page: 'home' };
    case 'board':
      return { page: 'board' };
    case 'map':
      return { page: 'map' };
    default:
      return { page: 'home' };
  }
}

export function formatRoute(r: Route): string {
  const e = encodeURIComponent;
  switch (r.page) {
    case 'project':
      return `#/project/${e(r.project)}`;
    case 'plan':
      return `#/plan/${e(r.plan)}/${r.tab}`;
    default:
      return `#/${r.page}`;
  }
}

/** Current route, derived from `location.hash`. */
export const route = readable<Route>(parseRoute(location.hash), (set) => {
  const on = () => set(parseRoute(location.hash));
  on();
  window.addEventListener('hashchange', on);
  return () => window.removeEventListener('hashchange', on);
});

export function navigate(r: Route) {
  location.hash = formatRoute(r);
}
