<script lang="ts">
  import { api } from '../lib/api';
  import { drawer } from '../lib/drawer';
  import type { PlanSummary } from '../lib/generated/PlanSummary';
  import type { SearchResults } from '../lib/generated/SearchResults';
  import { t } from '../lib/i18n';
  import { planStatus } from '../lib/plans';
  import { projectColor } from '../lib/projectColor';
  import { navigate } from '../lib/router';
  import { hl, searchWords } from '../lib/search';
  import { ensureTopics, topics } from '../lib/topics';
  import StatusChip from './StatusChip.svelte';

  /** Global search in the top bar: "/" or Ctrl/Cmd+K, ↑ ↓ Enter Esc. */
  const DEBOUNCE_MS = 120;
  const LIMIT = 8;
  const RECENT = 5;

  type Item =
    | { kind: 'plan'; plan: PlanSummary }
    | { kind: 'task'; row: SearchResults['tasks'][number] }
    | { kind: 'topic'; topic: SearchResults['topics'][number] };

  let input = $state<HTMLInputElement>();
  let root = $state<HTMLDivElement>();
  let q = $state('');
  let open = $state(false);
  let sel = $state(0);
  let results = $state<SearchResults | null>(null);
  /** The query `results` answer (highlighting and "no results" use it). */
  let answered = $state('');
  let recent = $state<PlanSummary[] | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Bumped per request; an answer is shown only if no newer request was made. */
  let seq = 0;

  async function search(text: string) {
    const my = ++seq;
    try {
      const r = await api.get<SearchResults>(
        `/api/search?q=${encodeURIComponent(text)}&limit=${LIMIT}`,
      );
      if (my !== seq) return;
      results = r;
      answered = text;
      sel = 0;
    } catch {
      if (my === seq) {
        results = { plans: [], tasks: [], topics: [] };
        answered = text;
      }
    }
  }

  async function loadRecent() {
    try {
      const all = await api.get<PlanSummary[]>('/api/plans');
      recent = [...all]
        .sort((a, b) => (b.created ?? '').localeCompare(a.created ?? ''))
        .slice(0, RECENT);
    } catch {
      recent = recent ?? [];
    }
  }

  function onInput(e: Event) {
    q = (e.currentTarget as HTMLInputElement).value;
    open = true;
    sel = 0;
    clearTimeout(timer);
    if (!q.trim()) {
      seq++; // drop answers still on their way
      results = null;
      void loadRecent();
      return;
    }
    const text = q;
    timer = setTimeout(() => void search(text), DEBOUNCE_MS);
  }

  function onFocus() {
    ensureTopics();
    open = true;
    if (!q.trim()) void loadRecent();
  }

  function reset() {
    clearTimeout(timer);
    seq++;
    open = false;
    q = '';
    results = null;
    sel = 0;
    input?.blur();
  }

  const empty = $derived(!q.trim());
  const groups = $derived.by(() => {
    if (empty) return recent ? [{ key: 'recent', items: recent.map(planItem) }] : [];
    if (!results) return [];
    return [
      { key: 'plans', items: results.plans.map(planItem) },
      { key: 'tasks', items: results.tasks.map((row): Item => ({ kind: 'task', row })) },
      { key: 'topics', items: results.topics.map((topic): Item => ({ kind: 'topic', topic })) },
    ].filter((g) => g.items.length);
  });
  function planItem(plan: PlanSummary): Item {
    return { kind: 'plan', plan };
  }
  const items = $derived(groups.flatMap((g) => g.items));
  const offsets = $derived(
    groups.map((_, i) => groups.slice(0, i).reduce((n, g) => n + g.items.length, 0)),
  );
  const words = $derived(empty ? [] : searchWords(answered));
  const none = $derived(!empty && results !== null && answered === q && !items.length);

  const heading = (key: string, n: number) =>
    key === 'recent' ? $t('search.recent') : $t(`search.${key}`, { n });
  const optionId = (i: number) => `search-opt-${i}`;
  const topicInfo = (tag: string) => $topics.find((x) => x.tag === tag);
  const groupName = (g: string) =>
    ['type', 'domain', 'tech'].includes(g) ? $t(`group.${g}`) : $t('group.other');
  const plansText = (n: number) => (n === 1 ? $t('map.planOne') : $t('map.planCount', { n }));
  const statusOf = (p: PlanSummary) =>
    planStatus(
      p.done_count,
      p.task_count,
      p.tasks.some((x) => x.status !== 'todo'),
    );

  function activate(item: Item | undefined) {
    if (!item) return;
    reset();
    if (item.kind === 'plan') navigate({ page: 'plan', plan: item.plan.id, tab: 'docs' });
    else if (item.kind === 'task')
      drawer.open({ type: 'task', plan: item.row.plan_id, task: item.row.task.id });
    else drawer.open({ type: 'topic', tag: item.topic.tag });
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      open = true;
      sel = Math.min(sel + 1, items.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      sel = Math.max(sel - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (open) activate(items[sel]);
    } else if (e.key === 'Escape') {
      // Esc belongs to the search box here (the drawer behind must stay as it is).
      e.stopPropagation();
      reset();
    }
  }

  // Keep the selected option visible while moving with the arrows.
  $effect(() => {
    if (open) document.getElementById(optionId(sel))?.scrollIntoView?.({ block: 'nearest' });
  });

  $effect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const el = document.activeElement as HTMLElement | null;
      const typing = !!el && (/^(INPUT|SELECT|TEXTAREA)$/.test(el.tagName) || el.isContentEditable);
      if ((e.key === '/' && !typing) || (e.key.toLowerCase() === 'k' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        input?.focus();
        input?.select();
      }
    };
    const onPointer = (e: PointerEvent) => {
      if (open && root && !root.contains(e.target as Node)) open = false;
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('pointerdown', onPointer);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('pointerdown', onPointer);
    };
  });
</script>

<div class="search" role="search" bind:this={root}>
  <span class="ic" aria-hidden="true">🔍</span>
  <input
    bind:this={input}
    value={q}
    oninput={onInput}
    onfocus={onFocus}
    onkeydown={onKey}
    placeholder={$t('search.placeholder')}
    aria-label={$t('search.label')}
    autocomplete="off"
    spellcheck="false"
    role="combobox"
    aria-expanded={open && items.length > 0}
    aria-controls="search-results"
    aria-autocomplete="list"
    aria-activedescendant={open && items[sel] ? optionId(sel) : undefined}
  />
  <kbd aria-hidden="true">/</kbd>
  {#if open}
    <div class="sres">
      {#if items.length}
        <div id="search-results" role="listbox" aria-label={$t('search.label')}>
          {#each groups as g, gi (g.key)}
            <div role="group" aria-label={heading(g.key, g.items.length)}>
              <div class="h" aria-hidden="true">{heading(g.key, g.items.length)}</div>
              {#each g.items as item, k (k)}
                {@const i = offsets[gi] + k}
                <div
                  id={optionId(i)}
                  class="sitem"
                  class:on={i === sel}
                  role="option"
                  aria-selected={i === sel}
                  tabindex="-1"
                  onpointerdown={(e) => e.preventDefault()}
                  onpointermove={() => sel !== i && (sel = i)}
                  onclick={() => activate(item)}
                  onkeydown={(e) => e.key === 'Enter' && activate(item)}
                >
                  {#if item.kind === 'plan'}
                    {@const st = statusOf(item.plan)}
                    <span class="ico" aria-hidden="true">📄</span>
                    <div class="m">
                      <b>{@html hl(item.plan.title, words)}</b>
                      <div class="s">
                        <span class="dot" style="background:{projectColor(item.plan.project)}"
                        ></span>
                        {$t('search.project')}
                        {@html hl(item.plan.project, words)} · {$t('plan.tasksDone', {
                          done: item.plan.done_count,
                          total: item.plan.task_count,
                        })}
                      </div>
                    </div>
                    {#if st}<span class="status st-{st}">{$t(`planStatus.${st}`)}</span>{/if}
                  {:else if item.kind === 'task'}
                    <span class="ico" aria-hidden="true">✓</span>
                    <div class="m">
                      <b>{@html hl(item.row.task.name, words)}</b>
                      <div class="s parent">
                        <span class="pt">📄 {item.row.plan_title}</span><span>
                          &nbsp;› {item.row.task.id}
                        </span>
                      </div>
                    </div>
                    <StatusChip status={item.row.task.status} />
                  {:else}
                    {@const info = topicInfo(item.topic.tag)}
                    <span class="ico" aria-hidden="true">🏷</span>
                    <div class="m">
                      <b>
                        {@html hl(
                          info?.label ?? item.topic.tag.split('/').pop() ?? item.topic.tag,
                          words,
                        )}
                      </b>
                      <div class="s">
                        {groupName(info?.group ?? item.topic.tag.split('/')[0])} · {plansText(
                          item.topic.plans,
                        )}
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
        </div>
      {:else if none}
        <div class="snone">
          {$t('search.none', { q: answered, examples: $t('search.examples') })}
        </div>
      {/if}
      <div class="sfoot">{$t('search.footer')}</div>
    </div>
  {/if}
</div>

<style>
  .search {
    position: relative;
    width: 100%;
  }
  input {
    width: 100%;
    font: inherit;
    padding: 7px 34px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    color: var(--ink);
  }
  input:focus {
    outline: 2px solid var(--accent);
    border-color: transparent;
    background: var(--panel);
  }
  .ic {
    position: absolute;
    left: 11px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--faint);
    pointer-events: none;
  }
  kbd {
    position: absolute;
    right: 9px;
    top: 50%;
    transform: translateY(-50%);
    font:
      11px ui-monospace,
      monospace;
    color: var(--faint);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 5px;
    pointer-events: none;
  }
  .sres {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow);
    max-height: 72vh;
    overflow: auto;
    padding: 6px;
    z-index: 30;
  }
  .h {
    margin: 10px 8px 4px;
    font-size: 11px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--faint);
    font-weight: 600;
  }
  .sitem {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px;
    border-radius: 8px;
    cursor: pointer;
  }
  .sitem.on {
    background: var(--accent-soft);
  }
  .m {
    flex: 1;
    min-width: 0;
  }
  .m b {
    font-weight: 600;
  }
  .s {
    font-size: 12px;
    color: var(--mute);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .parent {
    display: flex;
  }
  .pt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .parent span:last-child {
    flex: none;
  }
  .ico {
    width: 18px;
    text-align: center;
    flex: none;
    color: var(--mute);
  }
  .sitem :global(mark) {
    background: #ffe8a3;
    color: inherit;
    border-radius: 2px;
    padding: 0 1px;
  }
  .sfoot {
    font-size: 11.5px;
    color: var(--faint);
    padding: 8px 8px 4px;
    border-top: 1px solid var(--line);
    margin-top: 6px;
  }
  .snone {
    padding: 14px 8px;
    color: var(--mute);
  }
  @media (prefers-color-scheme: dark) {
    .sitem :global(mark) {
      background: rgba(255, 212, 59, 0.3);
    }
  }
</style>
