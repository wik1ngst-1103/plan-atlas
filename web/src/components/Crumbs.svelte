<script lang="ts">
  import { t } from '../lib/i18n';
  import { route } from '../lib/router';
</script>

<nav class="crumbs" aria-label="Breadcrumb">
  {#if $route.page === 'home'}
    <b>{$t('nav.overview')}</b>
  {:else}
    <a href="#/home">{$t('nav.overview')}</a><span class="sep">›</span>
    {#if $route.page === 'project'}
      <b>{$t('nav.project', { name: $route.project })}</b>
    {:else if $route.page === 'plan'}
      <a href="#/project/{$route.plan.split('-').slice(1, -1).join('-')}">
        {$t('nav.project', { name: $route.plan.split('-').slice(1, -1).join('-') })}
      </a><span class="sep">›</span><b>{$route.plan}</b>
    {:else if $route.page === 'board'}
      <b>{$t('nav.board')}</b>
    {:else}
      <b>{$t('nav.map')}</b>
    {/if}
  {/if}
</nav>

<style>
  .crumbs {
    flex: 1;
    min-width: 0;
    color: var(--mute);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  b {
    color: var(--ink);
    font-weight: 600;
  }
  .sep {
    margin: 0 8px;
    color: var(--faint);
  }
</style>
