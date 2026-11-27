<script lang="ts">
  import type { Section } from '../lib/generated/Section';
  import { t } from '../lib/i18n';

  /** "On this page": the plan's headings; clicking scrolls smoothly, the route never changes. */
  let { sections }: { sections: Section[] } = $props();

  const go = (id: string) =>
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
</script>

<nav class="toc" aria-label={$t('plan.onThisPage')}>
  <div class="kind">{$t('plan.onThisPage')}</div>
  {#each sections as s (s.anchor)}
    <button onclick={() => go(s.anchor)}>{s.title}</button>
    {#each s.children as c (c.anchor)}
      <button class="l2" onclick={() => go(c.anchor)}>{c.title}</button>
    {/each}
  {/each}
</nav>

<style>
  .toc {
    position: sticky;
    top: 20px;
    align-self: start;
    font-size: 13px;
    max-height: calc(100vh - 100px);
    overflow: auto;
  }
  .kind {
    margin-bottom: 8px;
  }
  button {
    display: block;
    width: 100%;
    text-align: left;
    font-size: 13px;
    background: none;
    border: 0;
    border-left: 2px solid var(--line);
    border-radius: 0;
    color: var(--mute);
    padding: 3px 0 3px 10px;
  }
  button:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  button.l2 {
    padding-left: 24px;
    font-size: 12.5px;
  }
  @media (max-width: 900px) {
    .toc {
      display: none;
    }
  }
</style>
