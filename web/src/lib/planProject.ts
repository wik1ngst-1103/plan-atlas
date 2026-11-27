import { api } from './api';

type Info = { project: string; title: string };
const cache = new Map<string, Info>();

async function info(planId: string): Promise<Info | null> {
  const hit = cache.get(planId);
  if (hit) return hit;
  try {
    const p = await api.get<Info>(`/api/plans/${encodeURIComponent(planId)}`);
    const v = { project: p.project, title: p.title };
    cache.set(planId, v);
    return v;
  } catch {
    return null;
  }
}

/** Project of a plan, from the plan's own `project` field (cached); null when unknown. */
export const planProject = async (planId: string) => (await info(planId))?.project ?? null;

/** Title of a plan (cached with the project); null when unknown. */
export const planTitle = async (planId: string) => (await info(planId))?.title ?? null;

export const clearPlanProjectCache = () => cache.clear();

/** Refresh the cache from a freshly loaded plan (keeps breadcrumbs current). */
export const rememberPlan = (planId: string, project: string, title: string) =>
  cache.set(planId, { project, title });
