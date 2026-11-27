<script lang="ts">
  import { t } from '../lib/i18n';
  import type { Topic } from '../lib/topics';

  /** Topics in use, grouped (kind of work / domain / technology), with plan counts. */
  let {
    topics,
    total,
    selected,
    onselect,
  }: {
    topics: Topic[];
    /** Number of plans on the map ("All plans" row). */
    total: number;
    selected: string | null;
    onselect: (tag: string | null) => void;
  } = $props();

  const ORDER = ['type', 'domain', 'tech'];
  const groups = $derived.by(() => {
    const used = topics.filter((x) => x.plans > 0);
    const keys = [...ORDER, ...new Set(used.map((x) => x.group).filter((g) => !ORDER.includes(g)))];
    return keys
      .map((g) => ({
        g,
        items: used
          .filter((x) => x.group === g)
          .sort((a, b) => b.plans - a.plans || a.label.localeCompare(b.label)),
      }))
      .filter((x) => x.items.length);
  });
  const groupName = (g: string) => (ORDER.includes(g) ? $t(`group.${g}`) : $t('group.other'));
</script>

<nav class="topics" aria-label={$t('map.topicsLabel')}>
  <button
    class="topic"
    class:on={!selected}
    aria-pressed={!selected}
    onclick={() => onselect(null)}
  >
    {$t('map.allPlans')}<span class="n">{total}</span>
  </button>
  {#each groups as grp (grp.g)}
    <h5>{groupName(grp.g)}</h5>
    {#each grp.items as x (x.tag)}
      <button
        class="topic"
        class:on={selected === x.tag}
        aria-pressed={selected === x.tag}
        title={x.desc !== x.label ? x.desc : undefined}
        onclick={() => onselect(selected === x.tag ? null : x.tag)}
      >
        {x.label}<span class="n">
          {x.plans === 1 ? $t('map.planOne') : $t('map.planCount', { n: x.plans })}
        </span>
      </button>
    {/each}
  {/each}
</nav>

<style>
  .topics {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 12px;
    align-self: start;
    display: flex;
    flex-direction: column;
  }
  h5 {
    margin: 10px 6px 4px;
    font-size: 11px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--faint);
  }
  .topic {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    font-size: 13px;
  }
  .topic:hover {
    background: var(--accent-soft);
  }
  .topic.on {
    background: var(--accent);
    color: #fff;
  }
  .n {
    opacity: 0.7;
    font-size: 12px;
    white-space: nowrap;
  }
  @media (prefers-color-scheme: dark) {
    .topic.on {
      color: #141413;
    }
  }
  @media (max-width: 900px) {
    .topics {
      max-height: 240px;
      overflow: auto;
    }
  }
</style>
