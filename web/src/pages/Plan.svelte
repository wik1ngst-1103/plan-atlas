<script lang="ts">
  import BoardColumns from '../components/BoardColumns.svelte';
  import DocView from '../components/DocView.svelte';
  import Toc from '../components/Toc.svelte';
  import TopicChip from '../components/TopicChip.svelte';
  import { ApiError, api } from '../lib/api';
  import type { Plan } from '../lib/generated/Plan';
  import type { PlanDetail } from '../lib/generated/PlanDetail';
  import type { TaskRow as TaskItem } from '../lib/generated/TaskRow';
  import type { TaskStatus } from '../lib/generated/TaskStatus';
  import { locale, t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { rememberPlan } from '../lib/planProject';
  import { formatDate, planStatus, uniqueAnchors } from '../lib/plans';
  import { onDataChange } from '../lib/refetch';
  import { route, type PlanTab } from '../lib/router';
  import { ensureTopics, topicLabels } from '../lib/topics';

  const id = $derived($route.page === 'plan' ? $route.plan : '');
  const tab = $derived<PlanTab>($route.page === 'plan' ? $route.tab : 'docs');

  let plan = $state<PlanDetail | null>(null);
  let error = $state<string | null>(null);
  let notFound = $state(false);
  let moveError = $state<string | null>(null);

  async function load(planId: string) {
    try {
      // One request: the detail carries related plans' titles in both directions.
      const p = await api.get<PlanDetail>(`/api/plans/${encodeURIComponent(planId)}`);
      if (planId !== id) return;
      plan = p;
      rememberPlan(p.id, p.project, p.title);
      error = null;
      notFound = false;
    } catch (e) {
      if (planId !== id) return;
      if (e instanceof ApiError && e.status === 404) notFound = true;
      else error = (e as Error).message;
    }
  }

  $effect(() => {
    const planId = id;
    plan = null;
    notFound = false;
    error = null;
    moveError = null;
    void load(planId);
    ensureTopics();
    return onDataChange(() => void load(planId), planId);
  });

  const sections = $derived(plan ? uniqueAnchors(plan.sections) : []);
  const done = $derived(plan ? plan.tasks.filter((x) => x.status === 'done').length : 0);
  const total = $derived(plan?.tasks.length ?? 0);
  const active = $derived($activity.some((a) => a.plan_id === id && a.agents.length > 0));
  const status = $derived(
    plan ? planStatus(done, total, active || plan.tasks.some((x) => x.status !== 'todo')) : null,
  );
  const items = $derived<TaskItem[]>(
    plan
      ? plan.tasks.map((task) => ({
          plan_id: plan!.id,
          plan_title: plan!.title,
          project: plan!.project,
          plan_tags: plan!.tags,
          task,
        }))
      : [],
  );
  const relations = $derived(
    plan
      ? [
          ...plan.related.map((r) => ({
            key: `rel.out.${r.kind}`,
            other: r.to,
            title: plan!.outgoing_titles[r.to] ?? r.to,
          })),
          ...plan.incoming.map((r) => ({
            key: `rel.in.${r.kind}`,
            other: r.from_id,
            title: r.from_title,
          })),
        ]
      : [],
  );

  /** Split a sentence like "This plan depends on {link} being done first" around the link. */
  const around = (key: string) => {
    const [before, after = ''] = $t(key, { link: '\u0000' }).split('\u0000');
    return { before, after };
  };

  async function move(item: TaskItem, next: TaskStatus) {
    moveError = null;
    try {
      const updated = await api.patch<Plan>(
        `/api/plans/${encodeURIComponent(item.plan_id)}/tasks/${encodeURIComponent(item.task.id)}`,
        { status: next },
      );
      if (plan && updated.id === plan.id) plan = { ...plan, ...updated };
    } catch (e) {
      moveError = $t('plan.moveFailed', { message: (e as Error).message });
    }
  }

  const warnText = (code: string) => {
    const k = `warn.${code}`;
    const s = $t(k);
    return s === k ? $t('warn.other') : s;
  };
</script>

<div class="page">
  {#if notFound}
    <div class="empty">
      {$t('plan.notFound')} <a href="#/home">{$t('plan.backHome')}</a>
    </div>
  {:else if error}
    <div class="empty">
      {$t('error.load', { message: error })}
      <button onclick={() => load(id)}>{$t('error.retry')}</button>
    </div>
  {:else if !plan}
    <p class="lead">{$t('common.loading')}</p>
  {:else}
    {#if plan.warnings.length}
      <div class="warnbox" role="note">
        <b>⚠ {$t('warn.title')}</b>
        <ul>
          {#each plan.warnings as w, i (i)}<li>{warnText(w.code)}</li>{/each}
        </ul>
        <details>
          <summary>{$t('warn.details')}</summary>
          <ul class="raw">
            {#each plan.warnings as w, i (i)}<li><code>{w.code}</code>: {w.message}</li>{/each}
          </ul>
        </details>
      </div>
    {/if}

    <div class="kind">{$t('plan.kind', { id: plan.id })}</div>
    <h1 class="page-title">
      {plan.title}
      {#if !plan.structured}
        <span class="badge" title={$t('plan.importedHint')}>{$t('plan.imported')}</span>
      {/if}
    </h1>
    <p class="lead">
      {$t(total ? 'plan.pageLead' : 'plan.pageLeadNoTasks', { project: plan.project })}
    </p>

    <div class="meta head-meta">
      {#if status}
        <span class="status st-{status}">{$t(`planStatus.${status}`)}</span>
        <span>{$t('plan.tasksDone', { done, total })}</span>
      {:else}
        <span>{$t('plan.noTaskList')}</span>
      {/if}
      {#if plan.created}<span>
          {$t('plan.created', { date: formatDate(plan.created, $locale) })}
        </span>{/if}
    </div>
    {#if total}
      <div class="bar head-bar"><b style="width:{(done / total) * 100}%"></b></div>
    {/if}
    {#if plan.tags.length}
      <div>
        {#each plan.tags as tag (tag)}<TopicChip {tag} label={$topicLabels[tag]} />{/each}
      </div>
    {/if}
    {#if relations.length}
      <div class="rel">
        {#each relations as r, i (i)}
          {@const s = around(r.key)}
          <div>
            🔗 {s.before}<a href="#/plan/{encodeURIComponent(r.other)}/docs">
              {r.title}
            </a>{s.after}
          </div>
        {/each}
      </div>
    {/if}

    <nav class="subtabs">
      <a class:on={tab === 'docs'} href="#/plan/{encodeURIComponent(plan.id)}/docs">
        📄 {$t('plan.tabDocs')}
      </a>
      <a class:on={tab === 'tasks'} href="#/plan/{encodeURIComponent(plan.id)}/tasks">
        ✅ {$t('plan.tabTasks')} ({total})
      </a>
    </nav>

    {#if tab === 'tasks'}
      {#if total}
        <p class="lead tasks-lead">{$t('plan.lead')}</p>
        {#if moveError}<div class="empty err">{moveError}</div>{/if}
        <BoardColumns {items} onMove={move} />
      {:else}
        <div class="empty tasks-lead">{$t('plan.noTasks')}</div>
      {/if}
    {:else}
      <div class="doclayout">
        <DocView planId={plan.id} {sections} tasks={plan.tasks} />
        <Toc {sections} />
      </div>
    {/if}
  {/if}
</div>

<style>
  .warnbox {
    background: var(--warn-soft);
    border: 1px solid var(--warn);
    border-radius: 10px;
    padding: 10px 14px;
    margin-bottom: 18px;
    font-size: 13px;
  }
  .warnbox b {
    color: var(--warn);
  }
  .warnbox ul {
    margin: 6px 0;
    padding-left: 20px;
  }
  .warnbox summary {
    cursor: pointer;
    color: var(--mute);
    font-size: 12.5px;
  }
  .raw {
    font-size: 12px;
    color: var(--mute);
  }
  .head-meta {
    margin: 8px 0;
  }
  .head-bar {
    max-width: 420px;
    margin-bottom: 10px;
  }
  .rel {
    background: var(--accent-soft);
    border-radius: 8px;
    padding: 8px 12px;
    margin-top: 12px;
    font-size: 13px;
  }
  .subtabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--line);
    margin: 22px 0 0;
  }
  .subtabs a {
    padding: 9px 14px;
    color: var(--mute);
    border-bottom: 2px solid transparent;
    font-weight: 500;
  }
  .subtabs a:hover {
    text-decoration: none;
    color: var(--ink);
  }
  .subtabs a.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .doclayout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 230px;
    gap: 40px;
    margin-top: 24px;
  }
  .tasks-lead {
    margin-top: 16px;
  }
  .err {
    color: var(--warn);
    margin-bottom: 12px;
  }
  @media (max-width: 900px) {
    .doclayout {
      grid-template-columns: 1fr;
    }
  }
</style>
