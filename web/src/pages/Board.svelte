<script lang="ts">
  import BoardColumns from '../components/BoardColumns.svelte';
  import { api } from '../lib/api';
  import {
    ALL,
    boardFilter,
    columnKey,
    columnsFor,
    defaultFilter,
    filterItems,
    isFiltered,
    topicGroups,
    withStatus,
    type BoardGroup,
  } from '../lib/board';
  import type { TaskRow as TaskItem } from '../lib/generated/TaskRow';
  import type { TaskStatus } from '../lib/generated/TaskStatus';
  import { t } from '../lib/i18n';
  import { onDataChange } from '../lib/refetch';
  import { toast } from '../lib/toast';
  import { ensureTopics, topicLabels } from '../lib/topics';

  let all = $state<TaskItem[] | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      all = await api.get<TaskItem[]>('/api/tasks');
      error = null;
    } catch (e) {
      // Keep showing the last board if a refetch fails.
      if (!all) error = (e as Error).message;
    }
  }

  $effect(() => {
    void load();
    ensureTopics();
    return onDataChange(() => void load());
  });

  const f = $derived($boardFilter);
  const items = $derived(all ? filterItems(all, f) : []);
  const projects = $derived(all ? [...new Set(all.map((i) => i.project))].sort() : []);
  const groups = $derived(all ? topicGroups(all, f.tag) : []);
  const columns = $derived(
    all && f.group !== 'status' ? columnsFor(f.group, all, $t, $topicLabels) : undefined,
  );
  const label = (tag: string) => $topicLabels[tag] ?? tag.split('/').pop() ?? tag;

  const set = (patch: Partial<typeof f>) => boardFilter.update((x) => ({ ...x, ...patch }));
  const clear = () => boardFilter.update((x) => ({ ...defaultFilter(), group: x.group }));

  async function move(item: TaskItem, next: TaskStatus) {
    const prev = item.task.status;
    all = withStatus(all ?? [], item.plan_id, item.task.id, next);
    try {
      await api.patch(
        `/api/plans/${encodeURIComponent(item.plan_id)}/tasks/${encodeURIComponent(item.task.id)}`,
        { status: next },
      );
      toast($t('drawer.movedTo', { name: item.task.name, status: $t(`status.${next}`) }));
    } catch (e) {
      all = withStatus(all ?? [], item.plan_id, item.task.id, prev);
      toast($t('board.moveFailed', { message: (e as Error).message }), { error: true });
    }
  }
</script>

<div class="page">
  <h1 class="page-title">{$t('board.title')}</h1>
  <p class="lead">{$t('board.lead')}</p>

  {#if error}
    <div class="empty">
      {$t('error.load', { message: error })}
      <button onclick={load}>{$t('error.retry')}</button>
    </div>
  {:else if !all}
    <p class="lead">{$t('common.loading')}</p>
  {:else if all.length === 0}
    <div class="empty">{$t('board.empty')}</div>
  {:else}
    <div class="toolbar">
      <label>
        {$t('board.filterProject')}
        <select value={f.project} onchange={(e) => set({ project: e.currentTarget.value })}>
          <option value={ALL}>{$t('board.all')}</option>
          {#each projects as p (p)}<option value={p}>{p}</option>{/each}
        </select>
      </label>
      <label>
        {$t('board.filterTopic')}
        <select value={f.tag} onchange={(e) => set({ tag: e.currentTarget.value })}>
          <option value={ALL}>{$t('board.all')}</option>
          {#each groups as g (g.group)}
            <optgroup label={$t(`group.${g.group}`)}>
              {#each g.tags as tag (tag)}<option value={tag}>{label(tag)}</option>{/each}
            </optgroup>
          {/each}
        </select>
      </label>
      <label>
        {$t('board.groupBy')}
        <select
          value={f.group}
          onchange={(e) => set({ group: e.currentTarget.value as BoardGroup })}
        >
          <option value="status">{$t('board.groupStatus')}</option>
          <option value="model">{$t('board.groupModel')}</option>
          <option value="domain">{$t('board.groupDomain')}</option>
        </select>
      </label>
      <span class="count">
        {$t('board.showing', { n: items.length, total: all.length })}
        {#if isFiltered(f)}
          <button onclick={clear}>{$t('board.clearFilter')}</button>
        {/if}
      </span>
    </div>
    {#if f.group !== 'status'}
      <p class="hint">{$t('board.dragHint')}</p>
    {/if}
    <BoardColumns
      {items}
      showPlan
      {columns}
      keyOf={(i) => columnKey(i, f.group)}
      onMove={f.group === 'status' ? move : undefined}
    />
  {/if}
</div>

<style>
  .toolbar {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 14px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 10px 14px;
  }
  .toolbar label {
    color: var(--mute);
    font-size: 13px;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  select {
    font: inherit;
    font-size: 13px;
    color: var(--ink);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 5px 10px;
    max-width: 220px;
  }
  .count {
    margin-left: auto;
    color: var(--mute);
    font-size: 13px;
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .hint {
    color: var(--mute);
    font-size: 12.5px;
    margin: -6px 0 10px;
  }
  @media (max-width: 900px) {
    .count {
      margin-left: 0;
    }
  }
</style>
