<script lang="ts">
  import Drawer from './components/Drawer.svelte';
  import SideNav from './components/SideNav.svelte';
  import TopBar from './components/TopBar.svelte';
  import { startLive } from './lib/live';
  import { route } from './lib/router';
  import Board from './pages/Board.svelte';
  import Home from './pages/Home.svelte';
  import Map from './pages/Map.svelte';
  import Plan from './pages/Plan.svelte';
  import Project from './pages/Project.svelte';

  $effect(() => startLive());
</script>

<div class="shell">
  <TopBar />
  <SideNav />
  <main>
    {#if $route.page === 'project'}
      <Project />
    {:else if $route.page === 'plan'}
      <Plan />
    {:else if $route.page === 'board'}
      <Board />
    {:else if $route.page === 'map'}
      <Map />
    {:else}
      <Home />
    {/if}
  </main>
</div>
<Drawer />

<style>
  .shell {
    display: grid;
    grid-template-columns: 232px 1fr;
    grid-template-rows: 56px 1fr;
    height: 100vh;
  }
  @media (max-width: 900px) {
    .shell {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto auto 1fr;
      height: auto;
    }
  }
</style>
