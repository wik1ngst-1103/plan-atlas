<script lang="ts">
  import { drawer } from '../lib/drawer';
  import { t } from '../lib/i18n';

  const top = $derived($drawer.at(-1));
  const heading = $derived(
    top?.type === 'activity' ? $t('live.activityTitle') : top ? $t(`drawer.${top.type}`) : '',
  );
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && drawer.close()} />

<div class="scrim" class:on={top} onclick={drawer.close} role="presentation"></div>
<aside class="drawer" class:on={top} aria-label={$t('drawer.label')}>
  <div class="dh">
    {#if $drawer.length > 1}
      <button onclick={drawer.back} title={$t('drawer.backTitle')}>{$t('drawer.back')}</button>
    {/if}
    <span class="sp">{heading}</span>
    <button onclick={drawer.close} title={$t('drawer.close')} aria-label={$t('drawer.close')}>
      ✕
    </button>
  </div>
  <!-- Tasks 14-16 render task / topic / plan / activity bodies here -->
  <div class="db"></div>
</aside>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s;
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
    width: min(500px, 100vw);
    background: var(--panel);
    box-shadow: var(--shadow);
    transform: translateX(105%);
    transition: transform 0.22s ease;
    display: flex;
    flex-direction: column;
    z-index: 10;
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
</style>
