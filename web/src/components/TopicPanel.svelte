<script lang="ts">
  import { api } from '../lib/api';
  import { ALL, boardFilter } from '../lib/board';
  import { drawer } from '../lib/drawer';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import type { TaskRow as TaskItem } from '../lib/generated/TaskRow';
  import { t } from '../lib/i18n';
  import { mapTopic } from '../lib/mapFocus';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import { navigate } from '../lib/router';
  import { ensureTopics, topics } from '../lib/topics';
  import StatusChip from './StatusChip.svelte';

  /** What a topic means, how much uses it, and where. */
  let { tag }: { tag: string } = $props();
  const enc = encodeURIComponent;

  let plans = $state<PlanSummary[] | null>(null);
  let tasks = $state<TaskItem[]>([]);
  let error = $state<string | null>(null);

  async function load() {
    try {
      const [p, ts] = await Promise.all([
        api.get<PlanSummary[]>(`/api/plans?tag=${enc(tag)}`),
        api.get<TaskItem[]>(`/api/tasks?tag=${enc(tag)}`),
      ]);
      plans = p;
      tasks = ts;
      error = null;
    } catch (e) {
      if (!plans) error = (e as Error).message;
    }
  }

  $effect(() => {
    void load();
    ensureTopics();
    return onDataChange(() => void load());
  });

  const info = $derived($topics.find((x) => x.tag === tag));
  const group = $derived(info?.group ?? tag.split('/')[0]);
  const label = $derived(info?.label ?? tag.split('/').pop() ?? tag);
  const projects = $derived(new Set((plans ?? []).map((p) => p.project)).size);

  const openPlan = (id: string) => {
    navigate({ page: 'plan', plan: id, tab: 'docs' });
    drawer.close(); // last: closing unmounts this panel
  };
  const filterBoard = () => {
    boardFilter.update((f) => ({ ...f, project: ALL, tag }));
    navigate({ page: 'board' });
    drawer.close(); // last: closing unmounts this panel
  };
  const viewOnMap = () => {
    mapTopic.set(tag);
    navigate({ page: 'map' });
    drawer.close(); // last: closing unmounts this panel
  };
</script>

<div class="kind">🏷 {$t('drawer.topicIn', { group: $t(`group.${group}`) })}</div>
<h2>{label}</h2>
{#if info?.desc && info.desc !== label}
  <p class="muted desc">{info.desc}</p>
{/if}

{#if error}
  <div class="empty">{$t('error.load', { message: error })}</div>
{:else if !plans}
  <p class="muted">{$t('common.loading')}</p>
{:else}
  <div class="stats">
    <div class="stat"><b>{plans.length}</b><span>{$t('home.statPlans')}</span></div>
    <div class="stat"><b>{tasks.length}</b><span>{$t('topic.statTasks')}</span></div>
    <div class="stat"><b>{projects}</b><span>{$t('home.statProjects')}</span></div>
  </div>

  {#if plans.length}
    <h3 class="sec">{$t('relation.related')}</h3>
    {#each plans as p (p.id)}
      <button class="mini" onclick={() => openPlan(p.id)}>
        <span class="dot" style="background:{projectColor(p.project)}"></span>
        <span class="grow">
          <b>{p.title}</b>
          <span class="meta">
            {p.project} · {$t('plan.tasksDone', { done: p.done_count, total: p.task_count })}
          </span>
        </span>
        <span class="chev" aria-hidden="true">›</span>
      </button>
    {/each}
  {/if}

  {#if tasks.length}
    <h3 class="sec">{$t('board.relatedTasks')}</h3>
    {#each tasks as x (`${x.plan_id}/${x.task.id}`)}
      <button
        class="mini"
        onclick={() =>
          drawer.open({ type: 'task', plan: x.plan_id, task: x.task.id }, { push: true })}
      >
        <span class="grow">
          <b>{x.task.name}</b>
          <span class="meta">📄 {x.plan_title}</span>
        </span>
        <StatusChip status={x.task.status} />
      </button>
    {/each}
  {/if}
{/if}

<div class="actions">
  <button onclick={filterBoard}>🗂 {$t('drawer.openBoard')}</button>
  <button onclick={viewOnMap}>🧭 {$t('drawer.viewOnMap')}</button>
</div>

<style>
  .desc {
    margin-top: 0;
  }
  .stats {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin: 12px 0 8px;
  }
  .stat {
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 10px 14px;
    flex: 1;
    min-width: 90px;
  }
  .stat b {
    display: block;
    font-size: 22px;
  }
  .stat span {
    color: var(--mute);
    font-size: 12px;
  }
  .grow b {
    display: block;
    font-weight: 600;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 18px;
    flex-wrap: wrap;
  }
</style>
