<script lang="ts">
  import { api } from '../lib/api';
  import { drawer } from '../lib/drawer';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import { t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import LiveNest from './LiveNest.svelte';

  /** Everything Claude is doing, grouped Project → Plan → Task → assistant. */
  let plans = $state<PlanSummary[]>([]);

  async function load() {
    try {
      plans = await api.get<PlanSummary[]>('/api/plans');
    } catch {
      /* titles fall back to plan ids */
    }
  }

  $effect(() => {
    void load();
    return onDataChange(() => void load());
  });

  const byId = $derived(Object.fromEntries(plans.map((p) => [p.id, p])));
  const groups = $derived.by(() => {
    const ids = [...new Set($activity.filter((a) => a.agents.length).map((a) => a.plan_id))];
    const out = new Map<string, string[]>();
    for (const id of ids) {
      const project = byId[id]?.project ?? '';
      out.set(project, [...(out.get(project) ?? []), id]);
    }
    return [...out.entries()].sort(([a], [b]) => a.localeCompare(b));
  });
  const names = (id: string) =>
    Object.fromEntries((byId[id]?.tasks ?? []).map((x) => [x.id, x.name]));
</script>

<h2>{$t('live.activityTitle')}</h2>
<p class="muted lead">{$t('activity.lead')}</p>

{#if !groups.length}
  <div class="empty">{$t('activity.empty')}</div>
{:else}
  {#each groups as [project, ids] (project)}
    <div class="box">
      <div class="kind">
        <span class="dot" style="background:{projectColor(project)}"></span>
        {project ? $t('nav.project', { name: project }) : $t('activity.unknownProject')}
      </div>
      {#each ids as id (id)}
        <div class="plan">
          <a href="#/plan/{encodeURIComponent(id)}/docs" onclick={() => drawer.close()}>
            📄 {byId[id]?.title ?? id} ›
          </a>
          <LiveNest planId={id} taskNames={names(id)} head={false} push />
        </div>
      {/each}
    </div>
  {/each}
{/if}

<style>
  .lead {
    margin-top: 0;
  }
  .plan + .plan {
    margin-top: 12px;
  }
  .plan a {
    font-weight: 700;
    color: var(--ink);
  }
</style>
