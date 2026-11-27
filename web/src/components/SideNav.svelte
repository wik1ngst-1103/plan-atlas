<script lang="ts">
  import { api } from '../lib/api';
  import type { ProjectSummary } from '../lib/generated/ProjectSummary';
  import { t } from '../lib/i18n';
  import { onDataChange } from '../lib/refetch';
  import { projectColor, setKnownProjects } from '../lib/projectColor';
  import { route } from '../lib/router';

  let projects = $state<ProjectSummary[]>([]);
  const load = () =>
    api.get<ProjectSummary[]>('/api/projects').then(
      (p) => {
        projects = p;
        setKnownProjects(p.map((x) => x.name));
      },
      () => {},
    );

  $effect(() => {
    void load();
    return onDataChange(() => void load());
  });
</script>

<aside class="side" aria-label={$t('nav.menu')}>
  <a class="nav" class:on={$route.page === 'home'} href="#/home">{$t('nav.overview')}</a>
  <a class="nav" class:on={$route.page === 'board'} href="#/board">{$t('nav.board')}</a>
  <a class="nav" class:on={$route.page === 'map'} href="#/map">{$t('nav.map')}</a>
  <h6>{$t('nav.projects')}</h6>
  <div class="proj">
    {#each projects as p (p.name)}
      <a
        class="nav"
        class:on={$route.page === 'project' && $route.project === p.name}
        href="#/project/{encodeURIComponent(p.name)}"
      >
        <span class="dot" style="background:{$projectColor(p.name)}"></span>{p.name}
        <span class="n">{p.done_count}/{p.task_count}</span>
      </a>
    {:else}
      <div class="none">{$t('nav.noProjects')}</div>
    {/each}
  </div>
</aside>

<style>
  .side {
    border-right: 1px solid var(--line);
    padding: 14px 12px;
    overflow: auto;
    background: var(--panel);
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    color: var(--ink);
    cursor: pointer;
  }
  .nav:hover {
    background: var(--accent-soft);
    text-decoration: none;
  }
  .nav.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .n {
    margin-left: auto;
    color: var(--faint);
    font-size: 12px;
    font-weight: 400;
  }
  h6 {
    font-size: 11px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--faint);
    margin: 20px 10px 6px;
  }
  .none {
    color: var(--faint);
    font-size: 12.5px;
    padding: 4px 10px;
  }
  @media (max-width: 900px) {
    .side {
      display: flex;
      flex-wrap: wrap;
      gap: 4px;
      border-right: 0;
      border-bottom: 1px solid var(--line);
    }
    h6,
    .proj {
      display: none;
    }
  }
</style>
