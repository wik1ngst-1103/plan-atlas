<script lang="ts">
  import { api } from '../lib/api';
  import { STATUSES, STATUS_ICON } from '../lib/board';
  import { drawer, showRaw } from '../lib/drawer';
  import type { LogEntry } from '../lib/generated/LogEntry';
  import type { Plan } from '../lib/generated/Plan';
  import type { PlanDetail } from '../lib/generated/PlanDetail';
  import type { TaskStatus } from '../lib/generated/TaskStatus';
  import { locale, t } from '../lib/i18n';
  import { activity, onEvent } from '../lib/live';
  import { pastStep, presentStep, roleLabel } from '../lib/liveText';
  import { renderMarkdown } from '../lib/markdown';
  import { rememberPlan } from '../lib/planProject';
  import { MODEL_COLOR, MODEL_NAME, taskEffort, taskModel } from '../lib/plans';
  import { onDataChange } from '../lib/refetch';
  import { toast } from '../lib/toast';
  import { ensureTopics, topicLabels } from '../lib/topics';
  import Pips from './Pips.svelte';
  import TopicChip from './TopicChip.svelte';

  /** Details of one task: status, what Claude is doing, difficulty, description, links. */
  let { plan: planId, task: taskId }: { plan: string; task: string } = $props();

  const LOG_LIMIT = 10;
  const enc = encodeURIComponent;

  let detail = $state<PlanDetail | null>(null);
  let error = $state<string | null>(null);
  let log = $state<LogEntry[]>([]);

  async function load() {
    try {
      const d = await api.get<PlanDetail>(`/api/plans/${enc(planId)}`);
      detail = d;
      rememberPlan(d.id, d.project, d.title);
      error = null;
    } catch (e) {
      if (!detail) error = (e as Error).message;
    }
  }
  async function loadLog() {
    try {
      log = await api.get<LogEntry[]>(
        `/api/plans/${enc(planId)}/tasks/${enc(taskId)}/log?limit=${LOG_LIMIT}`,
      );
    } catch {
      /* keep the last log */
    }
  }

  $effect(() => {
    void load();
    void loadLog();
    ensureTopics();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const offStep = onEvent('agent_step', (e) => {
      if (e.plan_id !== planId || e.task_id !== taskId) return;
      clearTimeout(timer);
      timer = setTimeout(loadLog, 300);
    });
    const offData = onDataChange(() => void load(), planId);
    return () => {
      clearTimeout(timer);
      offStep();
      offData();
    };
  });

  const task = $derived(detail?.tasks.find((x) => x.id === taskId) ?? null);
  const agents = $derived(
    [...($activity.find((a) => a.plan_id === planId && a.task_id === taskId)?.agents ?? [])].sort(
      (x, y) => y.at - x.at,
    ),
  );
  const model = $derived(task ? taskModel(task) : null);
  const effort = $derived(task ? taskEffort(task) : null);
  /** "Sonnet" for "claude-sonnet-4-5"; other names as they are. */
  const modelLabel = (m: string | null) =>
    m ? (Object.entries(MODEL_NAME).find(([k]) => m.toLowerCase().includes(k))?.[1] ?? m) : '';
  const nameOf = (id: string) => detail?.tasks.find((x) => x.id === id)?.name ?? id;
  const time = (ms: number) =>
    new Date(ms).toLocaleTimeString($locale === 'vi' ? 'vi-VN' : 'en-GB', {
      hour: '2-digit',
      minute: '2-digit',
    });

  /** Split a sentence around `{model}` so the model name can be styled. */
  const sentence = $derived.by(() => {
    if (!model) return null;
    const s = $t('diff.sentence', {
      model: '\u0000',
      desc: $t(`model.${model}`),
      effort: effort ? $t(`effortWord.${effort}`) : '',
    });
    const [before, after = ''] = s.split('\u0000');
    return { before, after };
  });

  async function setStatus(next: TaskStatus) {
    if (!task || task.status === next) return;
    const name = task.name;
    try {
      const updated = await api.patch<Plan>(`/api/plans/${enc(planId)}/tasks/${enc(taskId)}`, {
        status: next,
      });
      if (detail) detail = { ...detail, ...updated };
      toast($t('drawer.movedTo', { name, status: $t(`status.${next}`) }));
    } catch (e) {
      toast($t('board.moveFailed', { message: (e as Error).message }), { error: true });
    }
  }

  const openTask = (id: string) =>
    drawer.open({ type: 'task', plan: planId, task: id }, { push: true });
</script>

{#if error}
  <div class="empty">{$t('error.load', { message: error })}</div>
{:else if !detail}
  <p class="muted">{$t('common.loading')}</p>
{:else if !task}
  <div class="empty">{$t('drawer.taskNotFound')}</div>
{:else}
  <div class="kind">✅ {$t('drawer.task')} · {task.id}</div>
  <h2>{task.name}</h2>
  <div class="meta inplan">
    {$t('drawer.belongsTo')}
    <a href="#/plan/{enc(planId)}/docs" onclick={() => drawer.close()}>{detail.title}</a>
  </div>

  <div class="kind">{$t('drawer.status')}</div>
  <div class="seg" role="group" aria-label={$t('drawer.status')}>
    {#each STATUSES as s (s)}
      <button
        class="st-{s}"
        class:on={task.status === s}
        aria-pressed={task.status === s}
        onclick={() => setStatus(s)}
      >
        {STATUS_ICON[s]}
        {$t(`status.${s}`)}
      </button>
    {/each}
  </div>

  <div class="box">
    <div class="kind">🤖 {$t('live.activityTitle')}</div>
    {#if agents.length}
      <div class="muted small">
        {agents.length === 1 ? $t('drawer.agentsOne') : $t('drawer.agents', { n: agents.length })}
      </div>
      {#each agents as a (a.agent_id)}
        <div class="agentrow">
          <span class="live"></span>
          <div>
            <span class="who">{roleLabel($t, a.role)}</span>
            {#if modelLabel(a.model)}<span class="faint">({modelLabel(a.model)})</span>{/if}
            <div class="what">{a.step.icon} {presentStep($t, $locale, a.step)}</div>
            {#if $showRaw}<div class="raw">{a.step.raw}</div>{/if}
          </div>
        </div>
      {/each}
    {:else if task.status === 'done'}
      <div>✅ {$t('drawer.completed')}</div>
    {:else if log.length}
      <div class="muted">{$t('drawer.idle')}</div>
    {:else}
      <div class="muted">{$t('drawer.notStarted')}</div>
    {/if}
    {#if log.length}
      <div class="kind tlh">{$t('drawer.timeline')}</div>
      <ul class="tl">
        {#each log as e, i (`${e.at}-${i}`)}
          <li>
            <span class="time">{time(e.at)}</span>{e.step.icon}
            {roleLabel($t, e.role)}
            {pastStep($t, $locale, e.step)}
            {#if $showRaw}<div class="raw">{e.step.raw}</div>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if agents.length || log.length}
      <label class="rawtoggle">
        <input
          type="checkbox"
          checked={$showRaw}
          onchange={(e) => showRaw.set(e.currentTarget.checked)}
        />
        {$t('drawer.showRaw')}
      </label>
    {/if}
  </div>

  <div class="box">
    <div class="kind">{$t('drawer.difficulty')}</div>
    {#if task.complexity}
      <div class="lvl">
        <Pips n={task.complexity} />
        <b>{$t(`diff.${task.complexity}`)}</b>
        <span class="faint">({task.complexity}/5)</span>
      </div>
    {:else}
      <div class="muted">{$t('diff.none')}</div>
    {/if}
    {#if model && sentence}
      <div class="why">
        {sentence.before}<b style="color:{MODEL_COLOR[model]}">
          {MODEL_NAME[model]}
        </b>{sentence.after}
      </div>
    {/if}
  </div>

  <div class="box">
    <div class="kind">{$t('drawer.description')}</div>
    {#if task.body_md.trim()}
      <!-- renderMarkdown sanitizes -->
      <div class="md">{@html renderMarkdown(task.body_md)}</div>
    {:else}
      <div class="muted">{$t('drawer.noDescription')}</div>
    {/if}
    {#if task.depends_on.length}
      <div class="deps">
        {$t('drawer.dependsOn')}
        {#each task.depends_on as d, i (d)}
          {#if i > 0},
          {/if}<button class="linkbtn" onclick={() => openTask(d)}>{nameOf(d)}</button>
        {/each}
      </div>
    {/if}
  </div>

  {#if task.tags.length}
    <div class="kind">
      {$t('drawer.topic')} <span class="hintword">— {$t('drawer.otherPlans')}</span>
    </div>
    <div class="chips">
      {#each task.tags as tag (tag)}<TopicChip {tag} label={$topicLabels[tag]} />{/each}
    </div>
  {/if}
{/if}

<style>
  .inplan {
    margin-bottom: 14px;
  }
  .seg {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .seg button {
    flex: 1;
    border: 0;
    border-radius: 0;
    border-right: 1px solid var(--line);
    padding: 7px 4px;
    background: none;
  }
  .seg button:last-child {
    border-right: 0;
  }
  .seg button:not(.on) {
    color: var(--mute);
  }
  .seg button.on {
    font-weight: 700;
  }
  .seg button:not(.on).st-todo,
  .seg button:not(.on).st-in_progress,
  .seg button:not(.on).st-review,
  .seg button:not(.on).st-done {
    background: none;
  }
  .seg button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .small {
    font-size: 13px;
  }
  .faint {
    color: var(--faint);
    font-size: 12px;
  }
  .agentrow {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 8px 0;
  }
  .agentrow .live {
    margin-top: 7px;
  }
  .who {
    font-weight: 600;
  }
  .what {
    color: var(--run);
  }
  .tlh {
    margin-top: 10px;
  }
  .tl {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tl li {
    position: relative;
    padding: 0 0 12px 22px;
    font-size: 13px;
  }
  .tl li:before {
    content: '';
    position: absolute;
    left: 5px;
    top: 7px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--line);
  }
  .tl li:after {
    content: '';
    position: absolute;
    left: 8px;
    top: 16px;
    bottom: 0;
    width: 1px;
    background: var(--line);
  }
  .tl li:last-child:after {
    display: none;
  }
  .time {
    color: var(--faint);
    font-size: 12px;
    margin-right: 6px;
  }
  .raw {
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    color: var(--faint);
    word-break: break-all;
  }
  .rawtoggle {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12.5px;
    color: var(--mute);
    margin-top: 4px;
  }
  .lvl {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .why {
    margin-top: 6px;
    font-size: 13px;
  }
  .md :global(p:first-child) {
    margin-top: 0;
  }
  .md :global(p:last-child) {
    margin-bottom: 0;
  }
  .deps {
    margin-top: 8px;
    font-size: 13px;
    color: var(--mute);
  }
  .hintword {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
  }
  .chips {
    margin-top: 6px;
  }
</style>
