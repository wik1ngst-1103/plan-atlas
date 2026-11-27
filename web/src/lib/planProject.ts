import { api } from './api';

const cache = new Map<string, string>();

/** Project of a plan, from the plan's own `project` field (cached); null when unknown. */
export async function planProject(planId: string): Promise<string | null> {
  const hit = cache.get(planId);
  if (hit) return hit;
  try {
    const p = await api.get<{ project: string }>(`/api/plans/${encodeURIComponent(planId)}`);
    cache.set(planId, p.project);
    return p.project;
  } catch {
    return null;
  }
}

export const clearPlanProjectCache = () => cache.clear();
