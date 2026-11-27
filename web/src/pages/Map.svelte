<script lang="ts">
  import Graph from '../components/Graph.svelte';
  import TopicList from '../components/TopicList.svelte';
  import { api } from '../lib/api';
  import { drawer } from '../lib/drawer';
  import type { Graph as GraphData } from '../lib/generated/Graph';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import { t } from '../lib/i18n';
  import { mapTopic } from '../lib/mapFocus';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import { ensureTopics, refreshTopics, topics } from '../lib/topics';

  let graph = $state<GraphData | null>(null);
  let error = $state<string | null>(null);
  let showShared = $state(true);

  async function load() {
    try {
      graph = await api.get<GraphData>('/api/graph');
      error = null;
    } catch (e) {
      if (!graph) error = (e as Error).message;
    }
  }

  // Plans of the selected topic, from the server (a plan has a topic directly or via a task).
  let topicPlans = $state<PlanSummary[] | null>(null);
  let seq = 0;
  async function loadTopic(tag: string | null) {
    const my = ++seq;
    if (!tag) {
      topicPlans = null;
      return;
    }
    try {
      const ps = await api.get<PlanSummary[]>(`/api/plans?tag=${encodeURIComponent(tag)}`);
      if (my === seq) topicPlans = ps;
    } catch {
      if (my === seq) topicPlans = [];
    }
  }

  $effect(() => {
    void load();
    ensureTopics();
    return onDataChange(() => {
      void load();
      void refreshTopics(); // plan counts in the topic list
      void loadTopic($mapTopic);
    });
  });
  $effect(() => {
    void loadTopic($mapTopic);
  });

  const info = $derived($topics.find((x) => x.tag === $mapTopic));
  const label = (tag: string) =>
    $topics.find((x) => x.tag === tag)?.label ?? tag.split('/').pop() ?? tag;
  const highlight = $derived($mapTopic && topicPlans ? new Set(topicPlans.map((p) => p.id)) : null);
  const projects = $derived(graph ? [...new Set(graph.nodes.map((n) => n.project))].sort() : []);
  const openPlan = (id: string) => drawer.open({ type: 'plan', plan: id });
</script>

<div class="page">
  <h1 class="page-title">{$t('map.title')}</h1>
  <p class="lead">{$t('map.lead')}</p>

  {#if error}
    <div class="empty">
      {$t('error.load', { message: error })}
      <button onclick={load}>{$t('error.retry')}</button>
    </div>
  {:else if !graph}
    <p class="lead">{$t('common.loading')}</p>
  {:else if !graph.nodes.length}
    <div class="empty">{$t('map.empty')}</div>
  {:else}
    <div class="maplayout">
      <TopicList
        topics={$topics}
        total={graph.nodes.length}
        selected={$mapTopic}
        onselect={(tag) => mapTopic.set(tag)}
      />
      <div class="main">
        <Graph
          plans={graph.nodes}
          edges={graph.edges}
          {highlight}
          {showShared}
          topicLabel={label}
          onopen={openPlan}
        />
        <div class="legend">
          {#each projects as p (p)}
            <span>
              <svg width="12" height="12" aria-hidden="true">
                <circle cx="6" cy="6" r="6" style="fill:{$projectColor(p)}" />
              </svg>{p}
            </span>
          {/each}
          <span>
            <svg width="28" height="8" aria-hidden="true">
              <line class="l-dep" x1="0" y1="4" x2="28" y2="4" />
            </svg>{$t('map.legendDepends')}
          </span>
          <span>
            <svg width="28" height="8" aria-hidden="true">
              <line class="l-rel" x1="0" y1="4" x2="28" y2="4" />
            </svg>{$t('map.legendRelated')}
          </span>
          <span>
            <svg width="28" height="8" aria-hidden="true">
              <line class="l-shared" x1="0" y1="4" x2="28" y2="4" />
            </svg>{$t('map.legendShared')}
          </span>
          <span>{$t('map.legendRing')}</span>
          <label class="toggle">
            <input type="checkbox" bind:checked={showShared} />
            {$t('map.showShared')}
          </label>
        </div>
        {#if $mapTopic}
          <div class="hint">
            <b>{info?.label ?? label($mapTopic)}</b>{#if info?.desc && info.desc !== info.label}: {info.desc}{/if}
            {#if topicPlans}
              {' '}{$t('map.topicPlans', { n: topicPlans.length })}
              {#each topicPlans as p, i (p.id)}
                <button class="linkbtn" onclick={() => openPlan(p.id)}>{p.title}</button>{i <
                topicPlans.length - 1
                  ? ', '
                  : '.'}
              {/each}
            {/if}
          </div>
        {:else}
          <div class="hint">💡 {$t('map.hint')}</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .maplayout {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    gap: 16px;
  }
  .main {
    min-width: 0;
  }
  .legend {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    color: var(--mute);
    margin-top: 10px;
  }
  .legend svg {
    vertical-align: middle;
    margin-right: 4px;
  }
  .l-dep,
  .l-rel,
  .l-shared {
    stroke-width: 2;
  }
  .l-dep,
  .l-rel {
    stroke: var(--accent);
  }
  .l-rel {
    stroke-dasharray: 2 4;
  }
  .l-shared {
    stroke: var(--faint);
    stroke-dasharray: 5 5;
  }
  .toggle {
    margin-left: auto;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .hint {
    background: var(--accent-soft);
    color: var(--ink);
    border-radius: 10px;
    padding: 10px 14px;
    margin-top: 12px;
    font-size: 13px;
  }
  .linkbtn {
    border: 0;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: inherit;
  }
  .linkbtn:hover {
    text-decoration: underline;
  }
  @media (max-width: 900px) {
    .maplayout {
      grid-template-columns: minmax(0, 1fr);
    }
    .toggle {
      margin-left: 0;
    }
  }
</style>
