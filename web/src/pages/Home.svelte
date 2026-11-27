<script lang="ts">
  import PlanCard from '../components/PlanCard.svelte';
  import { api } from '../lib/api';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import type { ProjectSummary } from '../lib/generated/ProjectSummary';
  import type { TaskRow } from '../lib/generated/TaskRow';
  import { t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { liveFirst, runningCount, tasksByPlan } from '../lib/plans';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import { ensureTopics, refreshTopics, topicLabels } from '../lib/topics';

  let projects = $state<ProjectSummary[]>([]);
  let plans = $state<PlanSummary[]>([]);
  let rows = $state<TaskRow[]>([]);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  async function load() {
    try {
      [projects, plans, rows] = await Promise.all([
        api.get<ProjectSummary[]>('/api/projects'),
        api.get<PlanSummary[]>('/api/plans'),
        api.get<TaskRow[]>('/api/tasks'),
      ]);
      error = null;
    } catch (e) {
      error = (e as Error).message;
    }
    loaded = true;
  }

  $effect(() => {
    void load();
    ensureTopics();
    return onDataChange(() => {
      void load();
      void refreshTopics();
    });
  });

  const byPlan = $derived(tasksByPlan(rows));
  const groups = $derived.by(() => {
    const names = [...projects.map((p) => p.name)];
    for (const p of plans) if (!names.includes(p.project)) names.push(p.project);
    return names
      .map((name) => ({
        name,
        plans: liveFirst(
          plans.filter((p) => p.project === name),
          $activity,
        ),
      }))
      .filter((g) => g.plans.length);
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
        <span class="dot" style="background:{projectColor(g.name)}"></span>
        {$t('home.projectsHeading', { name: g.name })}
        <a class="more" href="#/project/{encodeURIComponent(g.name)}">{$t('common.viewProject')}</a>
      </h2>
      <div class="grid">
        {#each g.plans as p (p.id)}
          <PlanCard plan={p} tasks={byPlan[p.id]} labels={$topicLabels} />
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
