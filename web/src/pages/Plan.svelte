<script lang="ts">
  import { untrack } from 'svelte';
  import BoardColumns from '../components/BoardColumns.svelte';
  import DocView from '../components/DocView.svelte';
  import Icon from '../components/Icon.svelte';
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
  import { formatDate, planStatus, readingMinutes, uniqueAnchors } from '../lib/plans';
  import { projectColor } from '../lib/projectColor';
  import { onDataChange } from '../lib/refetch';
  import { route, type PlanTab } from '../lib/router';
  import { activeAnchor, flattenToc, readProgress, scrollParent } from '../lib/scrollspy';
  import { ensureTopics, topicLabels } from '../lib/topics';

  const id = $derived($route.page === 'plan' ? $route.plan : '');
  const tab = $derived<PlanTab>($route.page === 'plan' ? $route.tab : 'docs');
  const deepLink = $derived($route.page === 'plan' ? ($route.anchor ?? null) : null);

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
    // A new plan opens at its top, not at the previous plan's scroll position.
    untrack(() => {
      if (!deepLink) document.querySelector('main')?.scrollTo?.({ top: 0 });
    });
    void load(planId);
    ensureTopics();
    return onDataChange(() => void load(planId), planId);
  });

  const sections = $derived(plan ? uniqueAnchors(plan.sections) : []);
  const tocItems = $derived(flattenToc(sections));
  const minutes = $derived(readingMinutes(sections));
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

  // Scrollspy + reading progress. The document scrolls inside <main> on wide screens and with
  // the page on phones, so measure against whichever element actually scrolls.
  /** A heading counts as "being read" once its top passes this far below the visible top. */
  const SPY_OFFSET = 120;
  let activeId = $state<string | null>(null);
  let progress = $state(0);
  let farDown = $state(false);
  let menuOpen = $state(false);
  let lockUntil = 0;
  let article = $state<HTMLElement | null>(null);

  const scroller = () => scrollParent(article);
  const reduceMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    const items = tocItems;
    if (!article || !items.length) return;
    let raf = 0;
    const measure = () => {
      raf = 0;
      const sc = scroller();
      const top = sc ? sc.getBoundingClientRect().top : 0;
      const view = sc ? sc.clientHeight : innerHeight;
      const st = sc ? sc.scrollTop : scrollY;
      const height = sc ? sc.scrollHeight : document.documentElement.scrollHeight;
      progress = readProgress(st, height, view);
      farDown = st > view;
      if (Date.now() < lockUntil) return;
      const positions = [];
      for (const i of items) {
        const el = document.getElementById(i.anchor);
        if (el) positions.push({ anchor: i.anchor, top: el.getBoundingClientRect().top - top });
      }
      activeId = activeAnchor(positions, SPY_OFFSET, st + view >= height - 2, view);
    };
    const on = () => {
      if (!raf) raf = requestAnimationFrame(measure);
    };
    document.addEventListener('scroll', on, { capture: true, passive: true });
    window.addEventListener('resize', on);
    on();
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener('scroll', on, { capture: true });
      window.removeEventListener('resize', on);
    };
  });

  /** Scroll to a heading; highlight it at once rather than every section passed on the way. */
  function go(anchor: string) {
    const el = document.getElementById(anchor);
    if (!el) return;
    activeId = anchor;
    menuOpen = false;
    const smooth = !reduceMotion();
    lockUntil = Date.now() + (smooth ? 900 : 100);
    el.scrollIntoView({ behavior: smooth ? 'smooth' : 'auto', block: 'start' });
  }

  function toTop() {
    const sc = scroller();
    const opts: ScrollToOptions = { top: 0, behavior: reduceMotion() ? 'auto' : 'smooth' };
    if (sc) sc.scrollTo(opts);
    else window.scrollTo(opts);
  }

  // "#/plan/<id>/docs/<heading>" links land on that heading once the document is there.
  $effect(() => {
    const a = deepLink;
    if (!a || !article || tab !== 'docs') return;
    requestAnimationFrame(() => go(a));
  });

  const activeTitle = $derived(tocItems.find((i) => i.anchor === activeId)?.title ?? '');

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

    <header class="hero">
      <h1 class="hero-title">
        {plan.title}
        {#if !plan.structured}
          <span class="badge" title={$t('plan.importedHint')}>{$t('plan.imported')}</span>
        {/if}
      </h1>
      <p class="hero-lead">
        {$t(total ? 'plan.pageLead' : 'plan.pageLeadNoTasks', { project: plan.project })}
      </p>

      <ul class="facts">
        <li>
          <span class="dot" style="background:{$projectColor(plan.project)}"></span>
          <a href="#/project/{encodeURIComponent(plan.project)}">{plan.project}</a>
        </li>
        {#if status}
          <li class="progress">
            <span class="status st-{status}">{$t(`planStatus.${status}`)}</span>
            <span class="bar mini"><b style="width:{(done / total) * 100}%"></b></span>
            <span>{$t('plan.tasksDone', { done, total })}</span>
          </li>
        {:else}
          <li><Icon name="tasks" size={15} />{$t('plan.noTaskList')}</li>
        {/if}
        {#if plan.created}
          <li>
            <Icon name="calendar" size={15} />{$t('plan.created', {
              date: formatDate(plan.created, $locale),
            })}
          </li>
        {/if}
        <li><Icon name="clock" size={15} />{$t('plan.readTime', { n: minutes })}</li>
        <li>
          <Icon name="list" size={15} />{$t(
            sections.length === 1 ? 'plan.sectionsOne' : 'plan.sections',
            { n: sections.length },
          )}
        </li>
        <li class="pid" title={$t('plan.idHint')}>{plan.id}</li>
      </ul>

      {#if plan.tags.length}
        <div class="topics">
          {#each plan.tags as tag (tag)}<TopicChip {tag} label={$topicLabels[tag]} />{/each}
        </div>
      {/if}
      {#if relations.length}
        <div class="rel">
          {#each relations as r, i (i)}
            {@const s = around(r.key)}
            <div>
              <Icon name="link" size={14} />
              {s.before}<a href="#/plan/{encodeURIComponent(r.other)}/docs">
                {r.title}
              </a>{s.after}
            </div>
          {/each}
        </div>
      {/if}
    </header>

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
      <div class="readbar" aria-hidden="true"><b style="transform:scaleX({progress})"></b></div>
      <details class="tocmenu" bind:open={menuOpen}>
        <summary>
          <Icon name="list" size={16} />
          <span class="tm-label">{$t('plan.onThisPage')}</span>
          {#if activeTitle}<span class="tm-current">{activeTitle}</span>{/if}
          <span class="tm-chev"><Icon name="down" size={16} /></span>
        </summary>
        <Toc items={tocItems} active={activeId} onGo={go} inline />
      </details>
      <div class="doclayout">
        <div class="docwrap" bind:this={article}>
          <DocView planId={plan.id} {sections} tasks={plan.tasks} onGo={go} />
        </div>
        <aside class="rail">
          <Toc items={tocItems} active={activeId} onGo={go} />
        </aside>
      </div>
      {#if farDown}
        <button
          class="totop"
          onclick={toTop}
          title={$t('doc.backToTop')}
          aria-label={$t('doc.backToTop')}
        >
          <Icon name="up" size={18} />
        </button>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .warnbox {
    background: var(--warn-soft);
    border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent);
    border-radius: 10px;
    padding: 10px 14px;
    margin-bottom: 26px;
    max-width: 860px;
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
  .hero {
    max-width: 860px;
    padding: 6px 0 4px;
  }
  .hero-title {
    font-size: 32px;
    line-height: 1.2;
    font-weight: 700;
    letter-spacing: -0.022em;
    margin: 0 0 10px;
    text-wrap: balance;
    overflow-wrap: anywhere;
  }
  .hero-title .badge {
    position: relative;
    top: -3px;
  }
  .hero-lead {
    color: var(--mute);
    font-size: 16px;
    line-height: 1.6;
    margin: 0 0 18px;
    max-width: 64ch;
  }
  .facts {
    list-style: none;
    margin: 0 0 14px;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 22px;
    color: var(--mute);
    font-size: 13.5px;
  }
  .facts li {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .facts :global(.icon) {
    color: var(--faint);
  }
  .facts a {
    color: var(--ink);
    font-weight: 550;
  }
  .progress .mini {
    width: 72px;
    height: 5px;
  }
  .pid {
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 12px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .topics {
    margin-top: 2px;
  }
  .rel {
    background: var(--accent-soft);
    border-radius: 10px;
    padding: 10px 14px;
    margin-top: 12px;
    font-size: 13.5px;
    display: grid;
    gap: 4px;
  }
  .rel :global(.icon) {
    color: var(--accent);
    margin-right: 4px;
  }
  .subtabs {
    display: flex;
    gap: 4px;
    max-width: 1010px;
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
    grid-template-columns: minmax(0, 1fr) 240px;
    gap: 64px;
    max-width: 1010px;
    margin-top: 40px;
  }
  .docwrap {
    min-width: 0;
  }
  .rail {
    min-width: 0;
    display: flex;
  }
  .rail > :global(.toc) {
    width: 100%;
  }
  /* Reading progress: a hairline under the top bar. */
  .readbar {
    position: fixed;
    top: 56px;
    left: 232px;
    right: 0;
    height: 2px;
    z-index: 5;
    pointer-events: none;
  }
  .readbar b {
    display: block;
    height: 100%;
    background: var(--accent);
    transform-origin: 0 50%;
  }
  .tocmenu {
    display: none;
  }
  .totop {
    position: fixed;
    right: 28px;
    bottom: 28px;
    z-index: 6;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border-radius: 50%;
    color: var(--mute);
    background: var(--panel);
    border: 1px solid var(--line);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
  }
  .totop:hover {
    color: var(--accent);
  }
  .totop:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .tasks-lead {
    margin-top: 16px;
  }
  .err {
    color: var(--warn);
    margin-bottom: 12px;
  }
  @media (max-width: 1100px) {
    .doclayout {
      grid-template-columns: minmax(0, 1fr) 210px;
      gap: 36px;
    }
  }
  @media (max-width: 900px) {
    .hero-title {
      font-size: 25px;
    }
    .hero-lead {
      font-size: 15px;
    }
    .facts {
      gap: 6px 16px;
    }
    .doclayout {
      grid-template-columns: minmax(0, 1fr);
      margin-top: 20px;
      --anchor-offset: 72px;
    }
    .rail {
      display: none;
    }
    .readbar {
      top: 0;
      left: 0;
    }
    /* Phones: "On this page" as a menu that stays at the top while reading. */
    .tocmenu {
      display: block;
      position: sticky;
      top: 0;
      z-index: 4;
      margin: 16px -16px 0;
      padding: 0 16px;
      background: var(--bg);
      border-bottom: 1px solid var(--line);
    }
    .tocmenu summary {
      display: flex;
      align-items: center;
      gap: 8px;
      min-height: 48px;
      cursor: pointer;
      list-style: none;
      color: var(--mute);
      font-size: 13.5px;
    }
    .tocmenu summary::-webkit-details-marker {
      display: none;
    }
    .tm-label {
      flex: none;
      font-weight: 600;
      color: var(--ink);
    }
    .tm-current {
      flex: 1;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      color: var(--accent);
    }
    .tm-current::before {
      content: '·';
      margin-right: 8px;
      color: var(--faint);
    }
    .tm-chev {
      margin-left: auto;
      display: grid;
      transition: transform 0.2s ease;
    }
    .tocmenu[open] .tm-chev {
      transform: rotate(180deg);
    }
    /* The open list floats over the text, so opening it never shifts the document. */
    .tocmenu > :global(.toc) {
      position: absolute;
      top: 100%;
      left: 0;
      right: 0;
      padding: 4px 16px 14px;
      background: var(--bg);
      border-bottom: 1px solid var(--line);
      box-shadow: 0 14px 28px rgba(0, 0, 0, 0.18);
    }
    .totop {
      right: 16px;
      bottom: 16px;
    }
  }
</style>
