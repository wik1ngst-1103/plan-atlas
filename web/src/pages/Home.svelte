<script lang="ts">
  import PlanCard from '../components/PlanCard.svelte';
  import { api } from '../lib/api';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import { t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { liveFirst, runningCount } from '../lib/plans';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import { ensureTopics, topicLabels } from '../lib/topics';

  let plans = $state<PlanSummary[]>([]);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  /** One request: plan summaries already carry their tasks' names and statuses. */
  async function load() {
    try {
      plans = await api.get<PlanSummary[]>('/api/plans');
      error = null;
    } catch (e) {
      error = (e as Error).message;
    }
    loaded = true;
  }

  $effect(() => {
    void load();
    ensureTopics();
    return onDataChange(() => void load());
  });

  const groups = $derived.by(() => {
    const names = [...new Set(plans.map((p) => p.project))].sort();
    return names.map((name) => ({
      name,
      plans: liveFirst(
        plans.filter((p) => p.project === name),
        $activity,
      ),
    }));
  });
  const taskTotal = $derived(plans.reduce((n, p) => n + p.task_count, 0));
  const doneTotal = $derived(plans.reduce((n, p) => n + p.done_count, 0));
  const running = $derived(runningCount($activity));
</script>

<div class="page">
  <h1 class="page-title">{$t('home.title')}</h1>
  <p class="lead">{$t('home.lead')}</p>

  {#if error}
    <div class="empty">
      {$t('error.load', { message: error })}
      <button onclick={load}>{$t('error.retry')}</button>
    </div>
  {:else if !loaded}
    <p class="lead">{$t('common.loading')}</p>
  {:else if !plans.length}
    <div class="empty">{$t('home.empty')}</div>
  {:else}
    <div class="stats">
      <div class="stat"><b>{plans.length}</b><span>{$t('home.statPlans')}</span></div>
      <div class="stat"><b>{taskTotal}</b><span>{$t('home.statTasks')}</span></div>
      <div class="stat"><b>{doneTotal}</b><span>{$t('home.statDone')}</span></div>
      <div class="stat">
        <b class:run={running > 0}>{running}</b><span>{$t('home.statRunning')}</span>
      </div>
    </div>
    {#each groups as g (g.name)}
      <h2 class="sec">
        <span class="dot" style="background:{$projectColor(g.name)}"></span>
        {$t('home.projectsHeading', { name: g.name })}
        <a class="more" href="#/project/{encodeURIComponent(g.name)}">{$t('common.viewProject')}</a>
      </h2>
      <div class="grid">
        {#each g.plans as p (p.id)}
          <PlanCard plan={p} labels={$topicLabels} />
        {/each}
      </div>
    {/each}
  {/if}
</div>

<style>
  .stats {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }
  .stat {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 12px 16px;
    min-width: 130px;
  }
  .stat b {
    display: block;
    font-size: 22px;
  }
  .stat b.run {
    color: var(--run);
  }
  .stat span {
    color: var(--mute);
    font-size: 12px;
  }
  h2.sec {
    font-size: 15px;
    margin: 30px 0 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .more {
    font-weight: 400;
    font-size: 13px;
    margin-left: auto;
  }
</style>
