import { writable } from 'svelte/store';
import { api } from './api';

type Info = { project: string; title: string };
const cache = new Map<string, Info>();
const pending = new Map<string, Promise<Info | null>>();

/** Cached; concurrent callers share one request. */
function info(planId: string): Promise<Info | null> {
  const hit = cache.get(planId);
  if (hit) return Promise.resolve(hit);
  let p = pending.get(planId);
  if (!p) {
    p = api
      .get<Info>(`/api/plans/${encodeURIComponent(planId)}`)
      .then(
        (r) => {
          const v = { project: r.project, title: r.title };
          cache.set(planId, v);
          return v;
        },
        () => null,
      )
      .finally(() => pending.delete(planId));
    pending.set(planId, p);
  }
  return p;
}

/** Project of a plan, from the plan's own `project` field (cached); null when unknown. */
export const planProject = async (planId: string) => (await info(planId))?.project ?? null;

/** Title of a plan (cached with the project); null when unknown. */
export const planTitle = async (planId: string) => (await info(planId))?.title ?? null;

export const clearPlanProjectCache = () => {
  cache.clear();
  pending.clear();
};

/** Plans the app has loaded, by id (the breadcrumb reads this; no request of its own). */
export const knownPlans = writable<Record<string, Info>>({});

/** Record a freshly loaded plan (keeps breadcrumbs current). */
export function rememberPlan(planId: string, project: string, title: string) {
  cache.set(planId, { project, title });
  knownPlans.update((m) => ({ ...m, [planId]: { project, title } }));
}
