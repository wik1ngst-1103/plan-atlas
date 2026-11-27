<script lang="ts">
  import { drawer, type DrawerItem } from '../lib/drawer';
  import { t } from '../lib/i18n';
  import { knownPlans } from '../lib/planProject';
  import ActivityPanel from './ActivityPanel.svelte';
  import ImportPanel from './ImportPanel.svelte';
  import PlanPanel from './PlanPanel.svelte';
  import TaskPanel from './TaskPanel.svelte';
  import TopicPanel from './TopicPanel.svelte';

  /**
   * Right-side panel over the current page. Esc / ✕ / the scrim close it; "‹ Back" pops
   * the stack. Focus moves in on open and back to the opener on close.
   */
  const top = $derived($drawer.at(-1));
  const itemKey = (x: DrawerItem | undefined) =>
    !x
      ? ''
      : x.type === 'task'
        ? `task|${x.plan}|${x.task}`
        : x.type === 'topic'
          ? `topic|${x.tag}`
          : x.type === 'plan'
            ? `plan|${x.plan}`
            : x.type;
  const key = $derived(itemKey(top));
  const heading = $derived(
    !top
      ? ''
      : top.type === 'task'
        ? `📄 ${$knownPlans[top.plan]?.title ?? $t('drawer.task')}`
        : top.type === 'topic'
          ? $t('drawer.topic')
          : top.type === 'plan'
            ? $t('drawer.planSummary')
            : top.type === 'import'
              ? $t('import.title')
              : $t('live.activityTitle'),
  );

  let aside: HTMLElement;
  let body: HTMLElement;
  let opener: HTMLElement | null = null;
  let wasOpen = false;

  $effect(() => {
    const open = !!top;
    if (open && !wasOpen) {
      opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      aside.focus();
    } else if (!open && wasOpen) {
      if (opener?.isConnected) opener.focus();
      opener = null;
    }
    wasOpen = open;
  });

  // A different item starts at the top; live updates of the same item keep the scroll.
  $effect(() => {
    void key;
    if (body) body.scrollTop = 0;
  });

  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && $drawer.length) {
      e.preventDefault();
      drawer.close();
    }
  };
</script>

<svelte:window onkeydown={onKey} />

<div class="scrim" class:on={top} onclick={drawer.close} role="presentation"></div>
<aside
  class="drawer"
  class:on={top}
  aria-label={$t('drawer.label')}
  inert={!top}
  tabindex="-1"
  bind:this={aside}
>
  <div class="dh">
    {#if $drawer.length > 1}
      <button onclick={drawer.back} title={$t('drawer.backTitle')}>{$t('drawer.back')}</button>
    {/if}
    <span class="sp">{heading}</span>
    <button onclick={drawer.close} title={$t('drawer.close')} aria-label={$t('drawer.close')}>
      ✕
    </button>
  </div>
  <div class="db" bind:this={body}>
    {#key key}
      {#if top?.type === 'task'}
        <TaskPanel plan={top.plan} task={top.task} />
      {:else if top?.type === 'topic'}
        <TopicPanel tag={top.tag} />
      {:else if top?.type === 'plan'}
        <PlanPanel plan={top.plan} />
      {:else if top?.type === 'activity'}
        <ActivityPanel />
      {:else if top?.type === 'import'}
        <ImportPanel />
      {/if}
    {/key}
  </div>
</aside>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s;
    z-index: 9;
  }
  .scrim.on {
    opacity: 1;
    pointer-events: auto;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    height: 100vh;
    width: min(500px, 100%);
    background: var(--panel);
    box-shadow: var(--shadow);
    transform: translateX(105%);
    transition: transform 0.22s ease;
    display: flex;
    flex-direction: column;
    z-index: 10;
    outline: none;
  }
  .drawer.on {
    transform: none;
  }
  .dh {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--line);
  }
  .sp {
    flex: 1;
    min-width: 0;
    color: var(--mute);
    font-size: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .db {
    padding: 20px 22px 40px;
    overflow: auto;
    flex: 1;
  }
  /* Shared by the panels. */
  .db :global(h2) {
    margin: 2px 0 10px;
    font-size: 20px;
    line-height: 1.3;
  }
  .db :global(h3.sec) {
    font-size: 15px;
    margin: 26px 0 10px;
  }
  .db :global(.box) {
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 12px 14px;
    margin: 14px 0;
  }
  .db :global(.mini) {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 10px;
    border-radius: 8px;
    cursor: pointer;
    border: 1px solid var(--line);
    margin: 6px 0;
    width: 100%;
    text-align: left;
    background: none;
    font-size: 14px;
  }
  .db :global(.mini:hover),
  .db :global(.mini:focus-visible) {
    border-color: var(--accent);
    outline: none;
  }
  .db :global(.mini .grow) {
    flex: 1;
    min-width: 0;
  }
  .db :global(.chev) {
    color: var(--faint);
    font-size: 18px;
  }
  .db :global(.rel) {
    background: var(--accent-soft);
    border-radius: 8px;
    padding: 8px 12px;
    margin-top: 12px;
    font-size: 13px;
  }
  .db :global(.muted) {
    color: var(--mute);
  }
  .db :global(.linkbtn) {
    border: 0;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: inherit;
    cursor: pointer;
  }
  .db :global(.linkbtn:hover) {
    text-decoration: underline;
  }
</style>
