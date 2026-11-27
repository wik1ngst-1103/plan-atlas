<script lang="ts">
  import type { TaskRow as TaskItem } from '../lib/generated/TaskRow';
  import type { TaskStatus } from '../lib/generated/TaskStatus';
  import { t } from '../lib/i18n';
  import TaskCard from './TaskCard.svelte';

  const STATUSES: TaskStatus[] = ['todo', 'in_progress', 'review', 'done'];
  const ICON: Record<TaskStatus, string> = { todo: '○', in_progress: '◐', review: '◑', done: '●' };

  /**
   * Four status columns. With `onMove`, cards can be dragged to another column;
   * the callback gets the item and its new status.
   */
  let {
    items,
    showPlan = false,
    onMove,
  }: {
    items: TaskItem[];
    showPlan?: boolean;
    onMove?: (item: TaskItem, status: TaskStatus) => void;
  } = $props();

  let over = $state<TaskStatus | null>(null);

  const drop = (e: DragEvent, status: TaskStatus) => {
    e.preventDefault();
    over = null;
    const [plan, task] = (e.dataTransfer?.getData('text/plain') ?? '').split('|');
    const item = items.find((i) => i.plan_id === plan && i.task.id === task);
    if (item && item.task.status !== status) onMove?.(item, status);
  };
  const dragOver = (e: DragEvent, status: TaskStatus) => {
    if (!onMove) return;
    e.preventDefault();
    over = status;
  };
</script>

<div class="board">
  {#each STATUSES as st (st)}
    {@const list = items.filter((i) => i.task.status === st)}
    <section
      class="col"
      class:drop={over === st}
      aria-label={$t(`status.${st}`)}
      ondragover={(e) => dragOver(e, st)}
      ondragleave={() => over === st && (over = null)}
      ondrop={(e) => drop(e, st)}
    >
      <div class="hd">
        <span class="status st-{st}">{ICON[st]} {$t(`status.${st}`)}</span>
        <span class="n">{list.length}</span>
      </div>
      <div class="list">
        {#each list as item (`${item.plan_id}/${item.task.id}`)}
          <TaskCard {item} {showPlan} draggable={!!onMove} />
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
