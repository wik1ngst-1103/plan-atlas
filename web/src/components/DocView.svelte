<script lang="ts">
  import type { Section } from '../lib/generated/Section';
  import type { Task } from '../lib/generated/Task';
  import { locale, t } from '../lib/i18n';
  import { renderMarkdown } from '../lib/markdown';
  import { isTaskSection, taskIdOf } from '../lib/plans';
  import TaskRow from './TaskRow.svelte';

  /**
   * The plan document. `sections` must have unique anchors (headings get them as ids,
   * for the Toc). Section 04 lists the tasks as rows that open the task drawer.
   */
  let { planId, sections, tasks }: { planId: string; sections: Section[]; tasks: Task[] } =
    $props();

  /** Vietnamese name of a standard section ("01." … "05."), shown next to the English title. */
  const localName = (title: string) => {
    const m = /^0([1-5])\./.exec(title.trim());
    return m && $locale === 'vi' ? $t(`doc.s0${m[1]}`) : '';
  };
  const showTasks = (s: Section) => isTaskSection(s) && tasks.length > 0;
  const childrenOf = (s: Section) =>
    showTasks(s) ? s.children.filter((c) => !taskIdOf(c)) : s.children;
  const anchorFor = (s: Section, id: string) =>
    s.children.find((c) => taskIdOf(c) === id)?.anchor ?? `task-${id.toLowerCase()}`;
  const isEmpty = (s: Section) => !s.body_md.trim() && !s.children.length && !showTasks(s);

  /** In-document links: "#anchor" scrolls (the hash is the router's), others behave normally. */
  function localLinks(node: HTMLElement) {
    const on = (e: MouseEvent) => {
      const a = (e.target as Element).closest('a');
      const href = a?.getAttribute('href') ?? '';
      if (href.startsWith('#') && !href.startsWith('#/')) {
        e.preventDefault();
        document.getElementById(decodeURIComponent(href.slice(1)))?.scrollIntoView({
          behavior: 'smooth',
          block: 'start',
        });
      }
    };
    node.addEventListener('click', on);
    return { destroy: () => node.removeEventListener('click', on) };
  }
</script>

{#snippet heading(s: Section)}
  {#if s.level <= 2}
    <h2 id={s.anchor}>
      {s.title}{#if localName(s.title)}<small>{localName(s.title)}</small>{/if}
    </h2>
  {:else}
    <h3 id={s.anchor}>{s.title}</h3>
  {/if}
  {#if s.body_md.trim()}
    <div class="md">{@html renderMarkdown(s.body_md)}</div>
  {/if}
{/snippet}

<article class="doc" use:localLinks>
  {#each sections as s (s.anchor)}
    {@render heading(s)}
    {#if showTasks(s)}
      <p class="hint">{$t('plan.taskHint')}</p>
      {#each tasks as task (task.id)}
        <TaskRow {planId} {task} rowId={anchorFor(s, task.id)} />
      {/each}
    {/if}
    {#each childrenOf(s) as c (c.anchor)}
      {@render heading(c)}
    {/each}
    {#if isEmpty(s)}
      <p class="none">{$t('plan.none')}</p>
    {/if}
  {/each}
</article>

<style>
  .doc {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 8px 32px 28px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  h2 {
    font-size: 18px;
    margin: 28px 0 8px;
    scroll-margin-top: 20px;
  }
  h2 small {
    font-weight: 400;
    color: var(--faint);
    font-size: 13px;
    margin-left: 8px;
  }
  h3 {
    font-size: 15px;
    margin: 18px 0 4px;
    scroll-margin-top: 20px;
  }
  .hint {
    color: var(--mute);
    font-size: 13px;
    margin-top: 0;
  }
  .none {
    color: var(--faint);
  }
  .md :global(pre) {
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 12px;
    overflow: auto;
    font-size: 12.5px;
  }
  .md :global(code) {
    font-family: ui-monospace, monospace;
    font-size: 0.92em;
  }
  .md :global(:not(pre) > code) {
    background: var(--type);
    border-radius: 4px;
    padding: 0 4px;
  }
  .md :global(table) {
    border-collapse: collapse;
    display: block;
    overflow: auto;
  }
  .md :global(th),
  .md :global(td) {
    border: 1px solid var(--line);
    padding: 4px 8px;
    text-align: left;
  }
  .md :global(blockquote) {
    margin: 0;
    padding-left: 12px;
    border-left: 3px solid var(--line);
    color: var(--mute);
  }
  .md :global(h4),
  .md :global(h5),
  .md :global(h6) {
    font-size: 14px;
    margin: 14px 0 4px;
  }
  @media (max-width: 900px) {
    .doc {
      padding: 4px 16px 20px;
    }
  }
</style>
