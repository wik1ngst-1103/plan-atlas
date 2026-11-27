<script lang="ts">
  import PlanCard from '../components/PlanCard.svelte';
  import { api } from '../lib/api';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import { t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { liveFirst } from '../lib/plans';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import { route } from '../lib/router';
  import { ensureTopics, topicLabels } from '../lib/topics';

  const name = $derived($route.page === 'project' ? $route.project : '');
  let plans = $state<PlanSummary[]>([]);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  async function load(project: string) {
    try {
      const p = await api.get<PlanSummary[]>(`/api/plans?project=${encodeURIComponent(project)}`);
      if (project !== name) return;
      plans = p;
      error = null;
    } catch (e) {
      error = (e as Error).message;
    }
    loaded = true;
  }

  $effect(() => {
    const project = name;
    loaded = false;
    void load(project);
    ensureTopics();
    return onDataChange(() => void load(project));
  });

  const sorted = $derived(liveFirst(plans, $activity));
  const total = $derived(plans.reduce((n, p) => n + p.task_count, 0));
  const done = $derived(plans.reduce((n, p) => n + p.done_count, 0));
</script>

<div class="page">
  <div class="kind">
    <span class="dot" style="background:{$projectColor(name)}"></span>{$t('project.kind')}
  </div>
  <h1 class="page-title">{name}</h1>
  {#if error}
    <div class="empty">
      {$t('error.load', { message: error })}
      <button onclick={() => load(name)}>{$t('error.retry')}</button>
    </div>
  {:else if !loaded}
    <p class="lead">{$t('common.loading')}</p>
  {:else if !plans.length}
    <p class="lead">{$t('project.empty')}</p>
  {:else}
    <p class="lead">
      {$t('project.lead', {
        plans:
          plans.length === 1
            ? $t('project.planCountOne')
            : $t('project.planCount', { n: plans.length }),
        done,
        total,
      })}
    </p>
    <div class="grid">
      {#each sorted as p (p.id)}
        <PlanCard plan={p} labels={$topicLabels} />
      {/each}
    </div>
  {/if}
</div>
