<script lang="ts">
  import { drawer } from '../lib/drawer';
  import { locale, t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { presentStep, roleLabel } from '../lib/liveText';

  /**
   * What Claude is doing in one plan, nested plan → task → assistant.
   * Renders nothing when no task of the plan is active.
   */
  let {
    planId,
    taskNames = {},
    head = true,
  }: { planId: string; taskNames?: Record<string, string>; head?: boolean } = $props();

  const running = $derived(
    $activity
      .filter((a) => a.plan_id === planId && a.agents.length > 0)
      .map((a) => ({ ...a, agents: [...a.agents].sort((x, y) => y.at - x.at) })),
  );

  const open = (e: Event, task: string) => {
    e.stopPropagation();
    drawer.open({ type: 'task', plan: planId, task });
  };
  const key = (e: KeyboardEvent, task: string) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      open(e, task);
    }
  };
</script>

{#if running.length}
  <div class="nest">
    {#if head}
      <div class="nest-h">
        <span class="live"></span>
        {running.length === 1
          ? $t('live.planWorkingOne')
          : $t('live.planWorking', { n: running.length })}
      </div>
    {/if}
    {#each running as lt (lt.task_id)}
      <div
        class="ntask"
        role="button"
        tabindex="0"
        title={$t('live.openTask')}
        onclick={(e) => open(e, lt.task_id)}
        onkeydown={(e) => key(e, lt.task_id)}
      >
        <b>◐ {lt.task_id}{taskNames[lt.task_id] ? ` · ${taskNames[lt.task_id]}` : ''}</b>
        {#each lt.agents as a (a.agent_id)}
          <div class="nag">
            <span class="who">{roleLabel($t, a.role)}</span>
            {a.step.icon}
            {presentStep($t, $locale, a.step)}
          </div>
        {/each}
      </div>
    {/each}
  </div>
{/if}

<style>
  .nest {
    margin-top: 4px;
    border-top: 1px dashed var(--line);
    padding-top: 10px;
  }
  .nest-h {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 11px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--run);
    font-weight: 700;
    margin-bottom: 2px;
  }
  .ntask {
    padding: 6px 8px;
    margin: 0 -8px;
    border-radius: 8px;
    cursor: pointer;
  }
  .ntask:hover,
  .ntask:focus-visible {
    background: var(--run-soft);
    outline: none;
  }
  .ntask b {
    font-weight: 600;
    font-size: 13.5px;
  }
  .nag {
    position: relative;
    padding-left: 24px;
    font-size: 13px;
    color: var(--run);
    margin-top: 3px;
  }
  .nag:before {
    content: '';
    position: absolute;
    left: 7px;
    top: -5px;
    height: 14px;
    width: 11px;
    border-left: 1px solid var(--faint);
    border-bottom: 1px solid var(--faint);
    border-bottom-left-radius: 4px;
  }
  .who {
    color: var(--ink);
    font-weight: 600;
  }
</style>
