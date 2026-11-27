<script lang="ts">
  import { drawer } from '../lib/drawer';
  import type { TaskRow as TaskItem } from '../lib/generated/TaskRow';
  import { locale, t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { taskLiveText } from '../lib/liveText';
  import { MODEL_COLOR, MODEL_NAME, taskModel } from '../lib/plans';
  import { projectColor } from '../lib/projectColor';
  import Difficulty from './Difficulty.svelte';

  /** A task on a board. Click opens the task drawer (the board stays); drag to change status. */
  let {
    item,
    showPlan = false,
    draggable = false,
  }: { item: TaskItem; showPlan?: boolean; draggable?: boolean } = $props();

  const task = $derived(item.task);
  const model = $derived(taskModel(task));
  const liveText = $derived(
    taskLiveText(
      $t,
      $locale,
      $activity.find((a) => a.plan_id === item.plan_id && a.task_id === task.id),
    ),
  );
  let dragging = $state(false);

  const open = () => drawer.open({ type: 'task', plan: item.plan_id, task: task.id });
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      open();
    }
  };
  const start = (e: DragEvent) => {
    e.dataTransfer?.setData('text/plain', `${item.plan_id}|${task.id}`);
    dragging = true;
  };
</script>

<div
  class="card"
  class:dragging
  role="button"
  tabindex="0"
  title={$t('task.open')}
  {draggable}
  onclick={open}
  onkeydown={key}
  ondragstart={start}
  ondragend={() => (dragging = false)}
>
  {#if showPlan}
    <div class="pl">
      <span class="dot" style="background:{$projectColor(item.project)}"></span>📄 {item.plan_title}
    </div>
  {/if}
  <div class="ttl"><span class="id">{task.id}</span> {task.name}</div>
  {#if liveText}
    <div class="liveline"><span class="live"></span>{liveText}</div>
  {/if}
  <div class="ft">
    <Difficulty {task} short />
    {#if model}
      <span
        class="av"
        style="background:{MODEL_COLOR[model]}"
        title={$t('diff.modelHint', { model: MODEL_NAME[model], desc: $t(`model.${model}`) })}
      >
        {MODEL_NAME[model][0]}
      </span>
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 12px;
    margin-bottom: 8px;
    cursor: pointer;
  }
  .card:hover,
  .card:focus-visible {
    border-color: var(--accent);
    outline: none;
  }
  .card.dragging {
    opacity: 0.4;
  }
  .pl {
    font-size: 12px;
    color: var(--mute);
    display: flex;
    gap: 6px;
    align-items: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ttl {
    font-weight: 600;
    margin: 4px 0 6px;
    line-height: 1.35;
  }
  .id {
    color: var(--faint);
    font-weight: 500;
    font-size: 12px;
  }
  .liveline {
    color: var(--run);
    font-size: 13px;
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0 0 6px;
  }
  .ft {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    font-size: 12px;
    color: var(--mute);
  }
  .av {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 700;
    color: #fff;
    flex: none;
  }
</style>
