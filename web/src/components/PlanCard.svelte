<script lang="ts">
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import { t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { planStatus } from '../lib/plans';
  import { navigate } from '../lib/router';
  import LiveNest from './LiveNest.svelte';
  import TopicChip from './TopicChip.svelte';

  /** A plan as a big card: "📄 PLAN · project", progress, topics, and Claude's live work nested inside. */
  let { plan, labels = {} }: { plan: PlanSummary; labels?: Record<string, string> } = $props();

  const active = $derived($activity.some((a) => a.plan_id === plan.id && a.agents.length > 0));
  const status = $derived(
    planStatus(
      plan.done_count,
      plan.task_count,
      active || plan.tasks.some((x) => x.status !== 'todo'),
    ),
  );
  const pct = $derived(plan.task_count ? (plan.done_count / plan.task_count) * 100 : 0);
  const names = $derived(Object.fromEntries(plan.tasks.map((x) => [x.id, x.name])));

  const go = () => navigate({ page: 'plan', plan: plan.id, tab: 'docs' });
  const click = (e: MouseEvent) => {
    const hit = (e.target as Element).closest('button, a, [role="button"]');
    if (hit && hit !== e.currentTarget) return;
    go();
  };
  const key = (e: KeyboardEvent) => {
    if (e.target === e.currentTarget && e.key === 'Enter') go();
  };
</script>

<div
  class="plan-card"
  class:has-live={active}
  role="link"
  tabindex="0"
  aria-label={$t('card.open', { title: plan.title })}
  onclick={click}
  onkeydown={key}
>
  <div class="kind">{$t('card.kind', { project: plan.project })}</div>
  <h3>
    {plan.title}
    {#if !plan.structured}
      <span class="badge" title={$t('plan.importedHint')}>{$t('plan.imported')}</span>
    {/if}
  </h3>
  {#if status}
    <div class="bar"><b style="width:{pct}%"></b></div>
    <div class="meta">
      <span class="status st-{status}">{$t(`planStatus.${status}`)}</span>
      <span>{$t('plan.tasksDone', { done: plan.done_count, total: plan.task_count })}</span>
    </div>
  {:else}
    <div class="meta">{$t('plan.noTaskList')}</div>
  {/if}
  {#if plan.tags.some((g) => !g.startsWith('type/'))}
    <div>
      {#each plan.tags.filter((g) => !g.startsWith('type/')) as tag (tag)}
        <TopicChip {tag} label={labels[tag]} />
      {/each}
    </div>
  {/if}
  <LiveNest planId={plan.id} taskNames={names} />
</div>

<style>
  .plan-card {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 16px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition:
      border-color 0.15s,
      box-shadow 0.15s;
  }
  .plan-card:hover,
  .plan-card:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
    outline: none;
  }
  .plan-card.has-live {
    border-left: 3px solid var(--run);
  }
  h3 {
    margin: 0;
    font-size: 16px;
    line-height: 1.35;
  }
</style>
