<script lang="ts">
  import { t } from '../lib/i18n';
  import { tocLabel } from '../lib/plans';
  import type { TocItem } from '../lib/scrollspy';
  import Icon from './Icon.svelte';
  import Toc from './Toc.svelte';

  /**
   * "On this page" for phones: a sticky bar naming the section being read that opens the
   * list. Escape or a tap outside closes it; Escape hands focus back to the bar.
   */
  let {
    items,
    active,
    onGo,
    open = $bindable(false),
  }: {
    items: TocItem[];
    active: string | null;
    onGo: (anchor: string) => void;
    open?: boolean;
  } = $props();

  let menu = $state<HTMLDetailsElement | null>(null);
  let summary = $state<HTMLElement | null>(null);

  const current = $derived.by(() => {
    const item = items.find((i) => i.anchor === active);
    return item ? tocLabel(item.title) : '';
  });

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      open = false;
      summary?.focus();
    }
  }

  $effect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (menu && !menu.contains(e.target as Node)) open = false;
    };
    document.addEventListener('pointerdown', away);
    document.addEventListener('keydown', key);
    return () => {
      document.removeEventListener('pointerdown', away);
      document.removeEventListener('keydown', key);
    };
  });
</script>

<details class="tocmenu" bind:this={menu} bind:open>
  <summary bind:this={summary}>
    <Icon name="list" size={16} />
    <span class="tm-label">{$t('plan.onThisPage')}</span>
    {#if current}<span class="tm-current">{current}</span>{/if}
    <span class="tm-chev"><Icon name="down" size={16} /></span>
  </summary>
  <Toc {items} {active} {onGo} inline />
</details>

<style>
  .tocmenu {
    display: none;
  }
  @media (max-width: 900px) {
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
  }
  summary {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 48px;
    cursor: pointer;
    list-style: none;
    color: var(--mute);
    font-size: 13.5px;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    border-radius: 6px;
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
  @media (prefers-reduced-motion: reduce) {
    .tm-chev {
      transition: none;
    }
  }
</style>
