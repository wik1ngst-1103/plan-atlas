<script lang="ts">
  import { STATUSES, STATUS_ICON, type Column } from '../lib/board';
  import type { TaskRow as TaskItem } from '../lib/generated/TaskRow';
  import type { TaskStatus } from '../lib/generated/TaskStatus';
  import { t } from '../lib/i18n';
  import TaskCard from './TaskCard.svelte';

  /**
   * Task columns. By default the four status columns; pass `columns` + `keyOf` to group
   * by something else. With `onMove` (status columns only), cards can be dragged to
   * another column; the callback gets the item and its new status.
   */
  let {
    items,
    showPlan = false,
    onMove,
    columns,
    keyOf,
  }: {
    items: TaskItem[];
    showPlan?: boolean;
    onMove?: (item: TaskItem, status: TaskStatus) => void;
    columns?: Column[];
    keyOf?: (item: TaskItem) => string;
  } = $props();

  const byStatus = $derived(!columns);
  const cols = $derived<Column[]>(
    columns ?? STATUSES.map((s) => ({ key: s, label: `${STATUS_ICON[s]} ${$t(`status.${s}`)}` })),
  );
  const colOf = (i: TaskItem) => (keyOf && !byStatus ? keyOf(i) : i.task.status);
  const dnd = $derived(byStatus && !!onMove);

  let over = $state<string | null>(null);

  const drop = (e: DragEvent, key: string) => {
    if (!dnd) return;
    e.preventDefault();
    over = null;
    const [plan, task] = (e.dataTransfer?.getData('text/plain') ?? '').split('|');
    const item = items.find((i) => i.plan_id === plan && i.task.id === task);
    const status = key as TaskStatus;
    if (item && item.task.status !== status) onMove?.(item, status);
  };
  const dragOver = (e: DragEvent, key: string) => {
    if (!dnd) return;
    e.preventDefault();
    over = key;
  };
</script>

<div class="board">
  {#each cols as col (col.key)}
    {@const list = items.filter((i) => colOf(i) === col.key)}
    <section
      class="col"
      class:drop={over === col.key}
      aria-label={col.label}
      ondragover={(e) => dragOver(e, col.key)}
      ondragleave={() => over === col.key && (over = null)}
      ondrop={(e) => drop(e, col.key)}
    >
      <div class="hd">
        {#if byStatus}
          <span class="status st-{col.key}">{col.label}</span>
        {:else}
          <span>{col.label}</span>
        {/if}
        <span class="n">{list.length}</span>
      </div>
      <div class="list">
        {#each list as item (`${item.plan_id}/${item.task.id}`)}
          <TaskCard {item} {showPlan} draggable={dnd} />
        {:else}
          <div class="none">{$t('board.emptyColumn')}</div>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .board {
    display: flex;
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 8px;
    align-items: flex-start;
  }
  .col {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 10px;
    flex: 1 0 230px;
    min-width: 230px;
    display: flex;
    flex-direction: column;
  }
  .col.drop {
    outline: 2px dashed var(--accent);
    outline-offset: -2px;
  }
  .hd {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    font-weight: 600;
    border-bottom: 1px solid var(--line);
  }
  .n {
    color: var(--faint);
    font-weight: 400;
  }
  .list {
    padding: 8px;
    min-height: 70px;
  }
  .none {
    color: var(--faint);
    font-size: 12px;
    padding: 8px;
  }
</style>
