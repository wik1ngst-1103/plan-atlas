<script lang="ts">
  import { api } from '../lib/api';
  import { drawer } from '../lib/drawer';
  import type { PlanDetail } from '../lib/generated/PlanDetail';
  import type { Section } from '../lib/generated/Section';
  import { t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import { renderMarkdown } from '../lib/markdown';
  import { rememberPlan } from '../lib/planProject';
  import { planStatus } from '../lib/plans';
  import { onDataChange } from '../lib/refetch';
  import { navigate } from '../lib/router';
  import { ensureTopics, topicLabels } from '../lib/topics';
  import StatusChip from './StatusChip.svelte';
  import TopicChip from './TopicChip.svelte';

  /** Short summary of a plan (opened from the map); "Open full plan ›" navigates. */
  let { plan: planId }: { plan: string } = $props();

  let detail = $state<PlanDetail | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      const d = await api.get<PlanDetail>(`/api/plans/${encodeURIComponent(planId)}`);
      detail = d;
      rememberPlan(d.id, d.project, d.title);
      error = null;
    } catch (e) {
      if (!detail) error = (e as Error).message;
    }
  }

  $effect(() => {
    void load();
    ensureTopics();
    return onDataChange(() => void load(), planId);
  });

  /** First paragraph of the overview section (01), else of the first section with text. */
  function overview(sections: Section[]): string {
    const flat = (ss: Section[]): Section[] => ss.flatMap((s) => [s, ...flat(s.children)]);
    const all = flat(sections);
    const s =
      all.find((x) => x.title.trim().startsWith('01') && x.body_md.trim()) ??
      all.find((x) => x.body_md.trim());
    return (
      s?.body_md
        .split(/\n\s*\n/)
        .find((p) => p.trim() && !p.trim().startsWith('#'))
        ?.trim() ?? ''
    );
  }

  const done = $derived(detail ? detail.tasks.filter((x) => x.status === 'done').length : 0);
  const total = $derived(detail?.tasks.length ?? 0);
  const active = $derived($activity.some((a) => a.plan_id === planId && a.agents.length > 0));
  const status = $derived(
    detail
      ? planStatus(done, total, active || detail.tasks.some((x) => x.status !== 'todo'))
      : null,
  );
  const intro = $derived(detail ? overview(detail.sections) : '');
  const relations = $derived(
    detail
      ? [
          ...detail.related.map((r) => ({
            key: `rel.out.${r.kind}`,
            other: r.to,
            title: detail!.outgoing_titles[r.to] ?? r.to,
          })),
          ...detail.incoming.map((r) => ({
            key: `rel.in.${r.kind}`,
            other: r.from_id,
            title: r.from_title,
          })),
        ]
      : [],
  );
  const around = (key: string) => {
    const [before, after = ''] = $t(key, { link: '\u0000' }).split('\u0000');
    return { before, after };
  };

  const openFull = () => {
    navigate({ page: 'plan', plan: planId, tab: 'docs' });
    drawer.close(); // last: closing unmounts this panel
  };
</script>

{#if error}
  <div class="empty">{$t('error.load', { message: error })}</div>
{:else if !detail}
  <p class="muted">{$t('common.loading')}</p>
{:else}
  <div class="kind">{$t('card.kind', { project: detail.project })}</div>
  <h2>{detail.title}</h2>
  {#if total}
    <div class="meta">
      {#if status}<span class="status st-{status}">{$t(`planStatus.${status}`)}</span>{/if}
      {$t('plan.tasksDone', { done, total })}
    </div>
    <div class="bar progress"><b style="width:{(done / total) * 100}%"></b></div>
  {/if}
  {#if intro}
    <!-- renderMarkdown sanitizes -->
    <div class="intro">{@html renderMarkdown(intro)}</div>
  {/if}
  {#if detail.tags.length}
    <div>
      {#each detail.tags as tag (tag)}<TopicChip {tag} label={$topicLabels[tag]} />{/each}
    </div>
  {/if}
  {#each relations as r (`${r.key}|${r.other}`)}
    {@const s = around(r.key)}
    <div class="rel">
      🔗 {s.before}<button
        class="linkbtn"
        onclick={() => drawer.open({ type: 'plan', plan: r.other }, { push: true })}
      >
        {r.title}
      </button>{s.after}
    </div>
  {/each}

  {#if total}
    <h3 class="sec">{$t('plan.tasks')}</h3>
    {#each detail.tasks as x (x.id)}
      <button
        class="mini"
        onclick={() => drawer.open({ type: 'task', plan: planId, task: x.id }, { push: true })}
      >
        <span class="grow">
          <b>{x.name}</b>
          <span class="meta">
            {x.complexity
              ? $t('diff.label', { level: $t(`diff.${x.complexity}`) })
              : $t('diff.none')}
          </span>
        </span>
        <StatusChip status={x.status} />
      </button>
    {/each}
  {/if}

  <button class="primary" onclick={openFull}>{$t('drawer.openFullPlan')}</button>
{/if}

<style>
  .progress {
    margin: 10px 0;
  }
  .intro :global(p) {
    margin: 10px 0;
  }
  .grow b {
    display: block;
    font-weight: 600;
  }
  .primary {
    margin-top: 16px;
    background: var(--accent);
    color: #fff;
    border: 0;
    padding: 8px 14px;
  }
  @media (prefers-color-scheme: dark) {
    .primary {
      color: #141413;
    }
  }
</style>
