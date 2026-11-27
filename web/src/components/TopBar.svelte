<script lang="ts">
  import { drawer } from '../lib/drawer';
  import { locale, setLocale, t } from '../lib/i18n';
  import { activity } from '../lib/live';
  import Crumbs from './Crumbs.svelte';

  const working = $derived(new Set($activity.map((a) => `${a.plan_id}/${a.task_id}`)).size);
</script>

<header class="top">
  <div class="logo">🗺 {$t('app.name')}</div>
  <Crumbs />
  <!-- Task 16 mounts <Search /> here -->
  <div class="search-slot" id="search-slot"></div>
  {#if working > 0}
    <button class="livepill" onclick={() => drawer.open({ type: 'activity' })}>
      <span class="live"></span>{$t('live.working', { n: working })}
    </button>
  {/if}
  <div class="lang" role="group" aria-label={$t('topbar.language')}>
    <button
      class:on={$locale === 'en'}
      aria-pressed={$locale === 'en'}
      onclick={() => setLocale('en')}
    >
      EN
    </button>
    <button
      class:on={$locale === 'vi'}
      aria-pressed={$locale === 'vi'}
      onclick={() => setLocale('vi')}
    >
      VI
    </button>
  </div>
</header>

<style>
  .top {
    grid-column: 1 / 3;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .logo {
    font-weight: 700;
    font-size: 15px;
    width: 196px;
    flex: none;
  }
  .search-slot {
    flex: none;
    width: min(440px, 42vw);
  }
  .livepill {
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: 99px;
    background: var(--run-soft);
    color: var(--run);
    border: 0;
    font-weight: 600;
  }
  .lang {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
    flex: none;
  }
  .lang button {
    border: 0;
    border-radius: 0;
    background: none;
    padding: 5px 10px;
  }
  .lang button.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 700;
  }
  @media (max-width: 900px) {
    .top {
      grid-column: 1;
      flex-wrap: wrap;
      padding: 10px 16px;
    }
    .logo {
      width: auto;
    }
    .search-slot {
      width: 100%;
      order: 3;
    }
    .search-slot:empty {
      display: none;
    }
  }
</style>
