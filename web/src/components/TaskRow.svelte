<script lang="ts">
  import { drawer } from '../lib/drawer';
  import type { Task } from '../lib/generated/Task';
  import { locale, t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { taskLiveText } from '../lib/liveText';
  import Difficulty from './Difficulty.svelte';
  import StatusChip from './StatusChip.svelte';

  /** One task in the plan document. Opens the task drawer; never navigates. */
  let { planId, task, rowId }: { planId: string; task: Task; rowId?: string } = $props();

  const liveText = $derived(
    taskLiveText(
      $t,
      $locale,
      $activity.find((a) => a.plan_id === planId && a.task_id === task.id),
    ),
  );
  const open = () => drawer.open({ type: 'task', plan: planId, task: task.id });
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      open();
    }
  };
</script>

<div
  class="trow"
  id={rowId}
  role="button"
  tabindex="0"
  title={$t('task.open')}
  onclick={open}
  onkeydown={key}
>
  <div class="t">
    <b>{task.id} · {task.name}</b>
    <div class="sub">
      <Difficulty {task} />
      {#if liveText}
        <span class="lt"><span class="live"></span>{liveText}</span>
      {/if}
    </div>
  </div>
  <StatusChip status={task.status} />
  <span class="chev" aria-hidden="true">›</span>
</div>

<style>
  .trow {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    margin: 8px 0;
    cursor: pointer;
    scroll-margin-top: 20px;
  }
  .trow:hover,
  .trow:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
    outline: none;
  }
  .t {
    flex: 1;
    min-width: 0;
  }
  .t b {
    font-weight: 600;
  }
  .sub {
    color: var(--mute);
    font-size: 12.5px;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
  }
  .lt {
    color: var(--run);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .chev {
    color: var(--faint);
    font-size: 18px;
  }
</style>
