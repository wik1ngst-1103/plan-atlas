<script lang="ts">
  import { t } from '../lib/i18n';
  import { pathHeading, standardSection, tocLabel } from '../lib/plans';
  import { activeTrail, keepVisible, visibleToc, type TocItem } from '../lib/scrollspy';

  /**
   * "On this page": the plan's headings, with the one being read highlighted (`active`, from
   * the page's scrollspy). Clicking calls `onGo`; the route never changes. Long lists fold
   * sub-sections away except in the section being read. `inline` drops the sticky side-rail
   * look for the phone menu.
   */
  let {
    items,
    active,
    onGo,
    inline = false,
  }: {
    items: TocItem[];
    active: string | null;
    onGo: (anchor: string) => void;
    inline?: boolean;
  } = $props();

  const shown = $derived(visibleToc(items, active));
  const trail = $derived(activeTrail(items, active));
  const roots = $derived(items.filter((i) => i.depth === 0));
  const at = $derived(roots.findIndex((r) => trail.has(r.anchor)) + 1);

  let list = $state<HTMLElement | null>(null);

  // Keep the highlighted row in view when the list scrolls on its own.
  $effect(() => {
    const a = active;
    if (!list || !a || !list.clientHeight) return;
    const row = [...list.querySelectorAll<HTMLElement>('[data-anchor]')].find(
      (b) => b.dataset.anchor === a,
    );
    if (!row) return;
    const next = keepVisible(row.offsetTop, row.offsetHeight, list.scrollTop, list.clientHeight);
    if (next !== null) list.scrollTop = next;
  });
</script>

<nav class="toc" class:inline aria-label={$t('plan.onThisPage')}>
  {#if !inline}
    <div class="head">
      <span class="kind">{$t('plan.onThisPage')}</span>
      {#if at > 0 && roots.length > 1}
        <span class="count" title={$t('toc.progress', { n: at, total: roots.length })}>
          {at} / {roots.length}
        </span>
      {/if}
    </div>
  {/if}
  <ol bind:this={list}>
    {#each shown as i (i.anchor)}
      {@const std = standardSection(i.title)}
      <li>
        <button
          data-anchor={i.anchor}
          class="d{Math.min(i.depth, 2)}"
          class:on={i.anchor === active}
          class:trail={trail.has(i.anchor) && i.anchor !== active}
          class:mono={!!pathHeading(i.title)}
          aria-current={i.anchor === active ? 'location' : undefined}
          title={i.title}
          onclick={() => onGo(i.anchor)}
        >
          <span>
            {#if std}<i class="n" aria-hidden="true">{std.n}</i>{/if}{tocLabel(i.title)}
          </span>
        </button>
      </li>
    {/each}
  </ol>
</nav>

<style>
  .toc {
    font-size: 13px;
    min-width: 0;
  }
  .toc:not(.inline) {
    position: sticky;
    top: 24px;
    align-self: start;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 56px - 48px);
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    padding-left: 14px;
    margin-bottom: 10px;
  }
  .head .kind {
    margin: 0;
  }
  .count {
    font-size: 11.5px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    position: relative;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--line) transparent;
  }
  /* The guide line the rows hang from. */
  ol::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--line);
  }
  button {
    position: relative;
    display: block;
    width: 100%;
    text-align: left;
    font-size: 13px;
    line-height: 1.45;
    background: none;
    border: 0;
    border-radius: 0;
    color: var(--mute);
    padding: 5px 6px 5px 14px;
    overflow-wrap: anywhere;
    transition: color 0.15s ease;
  }
  button span {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .n {
    font-style: normal;
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--faint);
    margin-right: 8px;
  }
  button.on .n {
    color: inherit;
  }
  button.d1 {
    padding-left: 28px;
    font-size: 12.5px;
  }
  button.d2 {
    padding-left: 40px;
    font-size: 12.5px;
  }
  button.mono {
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 12px;
  }
  button::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 2px;
    border-radius: 2px;
    background: transparent;
    transition: background-color 0.15s ease;
  }
  button:hover {
    color: var(--ink);
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    border-radius: 4px;
  }
  button.trail {
    color: var(--ink);
    font-weight: 600;
  }
  button.trail::before {
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }
  button.on {
    color: var(--accent);
    font-weight: 600;
  }
  button.on::before {
    background: var(--accent);
  }
  .inline ol {
    max-height: min(60vh, 440px);
  }
  @media (prefers-reduced-motion: reduce) {
    button,
    button::before {
      transition: none;
    }
  }
</style>
