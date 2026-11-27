<script lang="ts">
  import type { Section } from '../lib/generated/Section';
  import type { Task } from '../lib/generated/Task';
  import { locale, t } from '../lib/i18n';
  import { renderMarkdown } from '../lib/markdown';
  import { isTaskSection, pathHeading, standardSection, taskIdOf } from '../lib/plans';
  import { formatRoute } from '../lib/router';
  import { toast } from '../lib/toast';
  import Icon from './Icon.svelte';
  import TaskRow from './TaskRow.svelte';

  /**
   * The plan document. `sections` must have unique anchors (headings get them as ids,
   * for the Toc). Section 04 lists the tasks as rows that open the task drawer.
   * `onGo` scrolls to an in-document anchor (the page owns scrolling and the scrollspy).
   */
  let {
    planId,
    sections,
    tasks,
    onGo,
  }: {
    planId: string;
    sections: Section[];
    tasks: Task[];
    onGo?: (anchor: string) => void;
  } = $props();

  /** Vietnamese name of a standard section ("01." … "05."), shown under the English title. */
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

  /** "server/src/a.ts" → ["server/src/", "a.ts"]: the folder is quieter than the file name. */
  const splitPath = (p: string) => {
    const i = p.replace(/\/$/, '').lastIndexOf('/');
    return i < 0 ? ['', p] : [p.slice(0, i + 1), p.slice(i + 1)];
  };

  let copied = $state<string | null>(null);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => () => clearTimeout(copiedTimer));
  async function copyLink(anchor: string) {
    const url = `${location.origin}${location.pathname}${formatRoute({
      page: 'plan',
      plan: planId,
      tab: 'docs',
      anchor,
    })}`;
    try {
      await navigator.clipboard.writeText(url);
      copied = anchor;
      toast($t('doc.linkCopied'));
      clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copied = null), 1600);
    } catch {
      toast($t('doc.linkCopyFailed'), { error: true });
    }
  }

  /** In-document links: "#anchor" scrolls (the hash is the router's), others behave normally. */
  function localLinks(node: HTMLElement) {
    const on = (e: MouseEvent) => {
      const a = (e.target as Element).closest('a');
      const href = a?.getAttribute('href') ?? '';
      if (href.startsWith('#') && !href.startsWith('#/')) {
        e.preventDefault();
        const id = decodeURIComponent(href.slice(1));
        if (onGo) onGo(id);
        else document.getElementById(id)?.scrollIntoView({ block: 'start' });
      }
    };
    node.addEventListener('click', on);
    return { destroy: () => node.removeEventListener('click', on) };
  }
</script>

{#snippet linkButton(anchor: string)}
  <button
    class="copy"
    class:done={copied === anchor}
    title={$t('doc.copyLink')}
    aria-label={$t('doc.copyLink')}
    onclick={() => copyLink(anchor)}
  >
    <Icon name={copied === anchor ? 'check' : 'link'} size={15} />
  </button>
{/snippet}

{#snippet body(s: Section)}
  {#if s.body_md.trim()}
    <!-- renderMarkdown sanitizes -->
    <div class="md">{@html renderMarkdown(s.body_md)}</div>
  {/if}
{/snippet}

{#snippet sub(s: Section)}
  {@const p = pathHeading(s.title)}
  {#if p}
    {@const [dir, base] = splitPath(p.path)}
    <div class="hrow h3row path">
      <h3 id={s.anchor} class="path" tabindex="-1">
        <span class="fileicon">
          <Icon name={p.path.endsWith('/') ? 'folder' : 'file'} size={16} />
        </span>
        <span class="ptext">
          {#if p.label}<span class="plabel">{p.label}</span>{/if}
          <code class="pname">
            <span class="dir">{dir}</span><wbr /><span class="base">{base}</span>
          </code>
          {#if p.note}<span class="pnote">{p.note}</span>{/if}
        </span>
      </h3>
      {@render linkButton(s.anchor)}
    </div>
  {:else}
    <div class="hrow h3row">
      <h3 id={s.anchor} tabindex="-1"><span class="htext">{s.title}</span></h3>
      {@render linkButton(s.anchor)}
    </div>
  {/if}
  {@render body(s)}
  {#each s.children as c (c.anchor)}
    {@render sub(c)}
  {/each}
{/snippet}

<article class="doc" use:localLinks>
  {#each sections as s (s.anchor)}
    {@const std = standardSection(s.title)}
    <section class="sec" class:std={!!std}>
      <div class="hrow h2row">
        <h2 id={s.anchor} tabindex="-1">
          {#if std}
            <span class="marker num" aria-hidden="true">{std.n}</span>
          {:else}
            <span class="marker" aria-hidden="true"><Icon name="hash" size={15} /></span>
          {/if}
          <span class="htext">
            {#if std}<span class="sr">{std.n}. </span>{std.name}{:else}{s.title}{/if}
            {#if localName(s.title)}<small>{localName(s.title)}</small>{/if}
          </span>
        </h2>
        {@render linkButton(s.anchor)}
      </div>
      {@render body(s)}
      {#if showTasks(s)}
        <p class="hint">{$t('plan.taskHint')}</p>
        <div class="tasks">
          {#each tasks as task (task.id)}
            <TaskRow {planId} {task} rowId={anchorFor(s, task.id)} />
          {/each}
        </div>
      {/if}
      {#each childrenOf(s) as c (c.anchor)}
        {@render sub(c)}
      {/each}
      {#if isEmpty(s)}
        <p class="none">{$t('plan.none')}</p>
      {/if}
    </section>
  {/each}
</article>

<style>
  .doc {
    min-width: 0;
    max-width: 70ch;
    font-size: 16.5px;
    line-height: 1.75;
    overflow-wrap: anywhere;
    color: var(--ink);
  }
  .sec + .sec {
    border-top: 1px solid var(--line);
    margin-top: 48px;
    padding-top: 40px;
  }

  /* Headings ---------------------------------------------------------------- */
  h2,
  h3 {
    position: relative;
    scroll-margin-top: var(--anchor-offset, 24px);
    text-wrap: balance;
  }
  h2 {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    font-size: 24px;
    line-height: 1.3;
    font-weight: 650;
    letter-spacing: -0.015em;
    margin: 0;
  }
  /* A heading and its copy-link button side by side (the button stays out of its name). */
  .hrow {
    display: flex;
    align-items: flex-start;
    gap: 4px;
  }
  .hrow > :is(h2, h3) {
    flex: 0 1 auto;
    min-width: 0;
  }
  .h2row {
    margin: 0 0 18px;
  }
  .h3row {
    margin: 40px 0 12px;
  }
  .h3row.path {
    margin-top: 44px;
  }
  h2:focus,
  h3:focus {
    outline: none;
  }
  .marker {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    margin-top: -1px;
    border-radius: 9px;
    background: var(--type);
    color: var(--faint);
  }
  .marker.num {
    background: var(--accent-soft);
    color: var(--accent);
    font:
      700 13px/1 ui-monospace,
      'SF Mono',
      Menlo,
      Consolas,
      monospace;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
  }
  .htext {
    min-width: 0;
  }
  h2 small {
    display: block;
    font-weight: 450;
    color: var(--faint);
    font-size: 14px;
    letter-spacing: 0;
    margin-top: 2px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  h3 {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 18.5px;
    line-height: 1.4;
    font-weight: 620;
    letter-spacing: -0.01em;
    margin: 0;
  }
  h3.path {
    gap: 10px;
    font-size: 16px;
    font-weight: 500;
  }
  .fileicon {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    background: var(--accent-soft);
    color: var(--accent);
    margin-top: -1px;
  }
  .ptext {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 10px;
  }
  .plabel {
    font-size: 11.5px;
    font-weight: 650;
    letter-spacing: 0.02em;
    color: var(--ok);
    background: var(--ok-soft);
    border-radius: 99px;
    padding: 1px 9px;
    align-self: center;
    white-space: nowrap;
  }
  .pname {
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 15px;
    font-weight: 600;
    color: var(--ink);
    overflow-wrap: anywhere;
  }
  .pname .dir {
    font-weight: 400;
    color: var(--mute);
  }
  /* Break inside the file name only when it cannot fit on a line of its own. */
  .pname .base {
    overflow-wrap: break-word;
  }
  .pnote {
    font-size: 15px;
    color: var(--mute);
    font-weight: 400;
  }

  /* Copy-link appears on hover / focus, at the end of the heading. */
  .copy {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    margin-left: 2px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--faint);
    opacity: 0;
    transition: opacity 0.15s ease;
  }
  .h2row .copy {
    margin-top: 2px;
  }
  .h3row .copy {
    margin-top: -1px;
  }
  .hrow:hover .copy,
  .copy:focus-visible,
  .copy.done {
    opacity: 1;
  }
  .copy:hover {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .copy.done {
    color: var(--ok);
  }
  .copy:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  @media (hover: none) {
    .copy {
      display: none;
    }
  }

  .hint {
    color: var(--mute);
    font-size: 14px;
    margin: 0 0 4px;
  }
  .tasks {
    margin: 0 0 8px;
  }
  .none {
    color: var(--faint);
    font-style: italic;
    margin: 0;
  }

  /* Markdown body ----------------------------------------------------------- */
  .md :global(> :first-child) {
    margin-top: 0;
  }
  .md :global(> :last-child) {
    margin-bottom: 0;
  }
  /* "---" closing a section: the section rule already separates it. */
  .md :global(> hr:last-child) {
    display: none;
  }
  .md :global(p) {
    margin: 0 0 1.05em;
  }
  .md :global(strong) {
    font-weight: 650;
  }
  .md :global(a) {
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 3px;
    text-decoration-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .md :global(a:hover) {
    text-decoration-color: currentColor;
  }
  .md :global(ul),
  .md :global(ol) {
    margin: 0 0 1.1em;
    padding-left: 1.5em;
  }
  .md :global(li) {
    margin: 0.4em 0;
    padding-left: 0.25em;
  }
  .md :global(li > ul),
  .md :global(li > ol) {
    margin: 0.3em 0 0.5em;
  }
  .md :global(li > p) {
    margin: 0 0 0.5em;
  }
  .md :global(ul > li::marker) {
    color: var(--faint);
  }
  .md :global(ol > li::marker) {
    color: var(--accent);
    font-weight: 650;
    font-size: 0.92em;
    font-variant-numeric: tabular-nums;
  }
  .md :global(h1),
  .md :global(h2),
  .md :global(h3),
  .md :global(h4),
  .md :global(h5),
  .md :global(h6) {
    font-size: 16px;
    font-weight: 650;
    line-height: 1.45;
    margin: 1.8em 0 0.5em;
  }
  .md :global(hr) {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 2em 0;
  }

  /* Inline code: soft pills that may wrap. Code blocks: their own box, scroll inside. */
  .md :global(code) {
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
  }
  .md :global(:not(pre) > code) {
    font-size: 0.84em;
    color: var(--typei);
    background: var(--type);
    border-radius: 5px;
    padding: 0.12em 0.42em;
    box-decoration-break: clone;
    -webkit-box-decoration-break: clone;
    overflow-wrap: anywhere;
  }
  .md :global(pre) {
    margin: 0 0 1.3em;
    padding: 16px 18px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 10px;
    overflow-x: auto;
    font-size: 13.5px;
    line-height: 1.65;
    tab-size: 2;
    overflow-wrap: normal;
    scrollbar-width: thin;
    scrollbar-color: var(--line) transparent;
  }
  .md :global(pre code) {
    white-space: pre;
  }

  /* Tables (wrapped by renderMarkdown): rounded, zebra rows, scroll inside the wrapper. */
  .md :global(.table-wrap) {
    margin: 0 0 1.4em;
    border: 1px solid var(--line);
    border-radius: 10px;
    overflow-x: auto;
    background: var(--panel);
    scrollbar-width: thin;
    scrollbar-color: var(--line) transparent;
  }
  .md :global(table) {
    width: 100%;
    border-collapse: collapse;
    font-size: 14.5px;
    line-height: 1.55;
  }
  .md :global(th),
  .md :global(td) {
    padding: 10px 14px;
    text-align: left;
    vertical-align: top;
    min-width: 12ch;
  }
  .md :global(th) {
    font-size: 13px;
    font-weight: 650;
    color: var(--mute);
    background: var(--type);
    white-space: nowrap;
  }
  .md :global(td) {
    border-top: 1px solid var(--line);
  }
  .md :global(tbody tr:nth-child(even) td) {
    background: color-mix(in srgb, var(--type) 45%, transparent);
  }
  .md :global(tbody tr:hover td) {
    background: var(--accent-soft);
  }

  /* Blockquotes read as callouts. */
  .md :global(blockquote) {
    position: relative;
    margin: 0 0 1.3em;
    padding: 14px 18px 14px 46px;
    background: var(--accent-soft);
    border-radius: 10px;
    color: var(--ink);
  }
  .md :global(blockquote::before) {
    content: '';
    position: absolute;
    left: 17px;
    top: 18px;
    width: 18px;
    height: 18px;
    background: var(--accent);
    -webkit-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round'%3E%3Ccircle cx='12' cy='12' r='9'/%3E%3Cpath d='M12 16v-4.5M12 8h.01'/%3E%3C/svg%3E")
      center / contain no-repeat;
    mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round'%3E%3Ccircle cx='12' cy='12' r='9'/%3E%3Cpath d='M12 16v-4.5M12 8h.01'/%3E%3C/svg%3E")
      center / contain no-repeat;
  }
  .md :global(blockquote > :last-child) {
    margin-bottom: 0;
  }
  .md :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
  }

  @media (max-width: 900px) {
    .doc {
      font-size: 15.5px;
      line-height: 1.7;
    }
    h2 {
      font-size: 21px;
      gap: 12px;
    }
    .marker {
      width: 28px;
      height: 28px;
    }
    h3 {
      font-size: 17px;
    }
    .sec + .sec {
      margin-top: 36px;
      padding-top: 30px;
    }
    .md :global(pre) {
      padding: 12px 14px;
      font-size: 12.5px;
    }
  }
</style>
