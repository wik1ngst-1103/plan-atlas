import { onEvent } from './live';

const DEBOUNCE_MS = 300;

/**
 * Call `cb` (debounced) when plans or tasks may have changed: plan_changed,
 * task_status, index_ready. `planId` limits plan/task events to one plan.
 * Returns the unsubscribe function.
 */
export function onDataChange(cb: () => void, planId?: string): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const soon = () => {
    clearTimeout(timer);
    timer = setTimeout(cb, DEBOUNCE_MS);
  };
  const mine = (e: { plan_id: string }) => {
    if (!planId || e.plan_id === planId) soon();
  };
  const offs = [
    onEvent('plan_changed', mine),
    onEvent('task_status', mine),
    onEvent('index_ready', soon),
  ];
  return () => {
    clearTimeout(timer);
    offs.forEach((off) => off());
  };
}
