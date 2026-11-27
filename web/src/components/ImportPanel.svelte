<script lang="ts">
  import { api } from '../lib/api';
  import { drawer } from '../lib/drawer';
  import type { ImportReport } from '../lib/generated/ImportReport';
  import type { ProjectSummary } from '../lib/generated/ProjectSummary';
  import { t } from '../lib/i18n';
  import {
    buildImportForm,
    formatSize,
    limitProblem,
    relPath,
    splitSelection,
    totalSize,
  } from '../lib/importFiles';
  import { projectColor } from '../lib/projectColor';
  import { navigate } from '../lib/router';

  /** Pick `.md` plans (files or a whole folder), choose a project, upload them. */
  const NEW = '__new';
  const PREVIEW = 30;

  let projects = $state<string[]>([]);
  let files = $state<File[]>([]);
  let ignored = $state(0);
  let choice = $state(''); // '' = Unsorted, NEW = typed name, else a project
  let newName = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let report = $state<ImportReport | null>(null);
  let fileInput: HTMLInputElement;
  let folderInput: HTMLInputElement;

  $effect(() => {
    api.get<ProjectSummary[]>('/api/projects').then(
      (p) => (projects = p.map((x) => x.name).filter((n) => n !== 'unsorted')),
      () => {},
    );
  });

  const onPick = (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const picked = splitSelection(Array.from(input.files ?? []));
    files = picked.md;
    ignored = picked.ignored;
    report = null;
    error = null;
  };

  const project = $derived(choice === NEW ? newName.trim() : choice);
  const problem = $derived(limitProblem(files));
  const problemText = $derived(
    !problem
      ? ''
      : problem.kind === 'tooMany'
        ? $t('import.tooMany', { max: problem.max })
        : problem.kind === 'tooBig'
          ? $t('import.tooBig', { name: problem.name })
          : $t('import.tooBigTotal'),
  );
  const canImport = $derived(
    files.length > 0 && !problem && !busy && (choice !== NEW || project !== ''),
  );

  async function submit() {
    if (!canImport) return;
    busy = true;
    error = null;
    try {
      report = await api.upload<ImportReport>('/api/import', buildImportForm(files, project));
      files = [];
      ignored = 0;
      fileInput.value = '';
      folderInput.value = '';
      // A new project now exists: offer it in the list and keep it selected.
      const created = report.plans[0]?.project;
      if (choice === NEW && created && created !== 'unsorted') {
        if (!projects.includes(created)) projects = [...projects, created].sort();
        choice = created;
        newName = '';
      }
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }

  const projectLabel = (p: string) => (p === 'unsorted' ? $t('import.unsorted') : p);
  const summary = $derived.by(() => {
    if (!report) return '';
    const into = [...new Set(report.plans.map((p) => p.project))].map(projectLabel).join(', ');
    const parts = [
      report.imported === 0
        ? $t('import.doneNone')
        : report.imported === 1
          ? $t('import.doneOne', { project: into })
          : $t('import.done', { n: report.imported, project: into }),
    ];
    if (report.skipped > 0) parts.push($t('import.skipped', { n: report.skipped }));
    return parts.join(' ');
  });

  const openPlan = (id: string) => {
    navigate({ page: 'plan', plan: id, tab: 'docs' });
    drawer.close(); // last: closing unmounts this panel
  };
</script>

<div class="kind">⤓ {$t('import.kind')}</div>
<h2>{$t('import.title')}</h2>
<p class="muted lead">{$t('import.lead')}</p>

<div class="pickers">
  <label class="pick">
    <input
      bind:this={fileInput}
      type="file"
      multiple
      accept=".md,text/markdown"
      aria-label={$t('import.chooseFiles')}
      onchange={onPick}
    />
    <span aria-hidden="true">📄 {$t('import.chooseFiles')}</span>
  </label>
  <label class="pick">
    <input
      bind:this={folderInput}
      type="file"
      webkitdirectory
      multiple
      aria-label={$t('import.chooseFolder')}
      onchange={onPick}
    />
    <span aria-hidden="true">📁 {$t('import.chooseFolder')}</span>
  </label>
</div>

<div class="box preview">
  {#if files.length}
    <div class="count">
      {files.length === 1
        ? $t('import.selectedOne', { size: formatSize(totalSize(files)) })
        : $t('import.selected', { n: files.length, size: formatSize(totalSize(files)) })}
    </div>
    <ul class="list">
      {#each files.slice(0, PREVIEW) as f (relPath(f))}
        <li>
          <span class="path">{relPath(f)}</span><span class="size">{formatSize(f.size)}</span>
        </li>
      {/each}
    </ul>
    {#if files.length > PREVIEW}
      <div class="muted small">{$t('import.more', { n: files.length - PREVIEW })}</div>
    {/if}
  {:else}
    <div class="muted">{$t('import.none')}</div>
  {/if}
  {#if ignored > 0}
    <div class="note">
      {ignored === 1 ? $t('import.ignoredOne') : $t('import.ignored', { n: ignored })}
    </div>
  {/if}
  {#if problemText}
    <div class="problem" role="alert">{problemText}</div>
  {/if}
</div>

<div class="field">
  <label for="import-project">{$t('import.project')}</label>
  <select id="import-project" bind:value={choice}>
    <option value="">{$t('import.unsorted')}</option>
    {#each projects as p (p)}
      <option value={p}>{p}</option>
    {/each}
    <option value={NEW}>{$t('import.newProject')}</option>
  </select>
  {#if choice === NEW}
    <input
      class="name"
      type="text"
      maxlength="100"
      aria-label={$t('import.newProjectName')}
      placeholder={$t('import.newProjectName')}
      bind:value={newName}
    />
  {/if}
</div>

<button class="go" onclick={submit} disabled={!canImport}>
  {busy ? $t('import.busy') : $t('import.submit')}
</button>

{#if error}
  <div class="problem" role="alert">{$t('import.error', { message: error })}</div>
{/if}

{#if report}
  <div class="result" role="status">
    <p class="ok">{summary}</p>
    {#if report.failed > 0}
      <p class="problem">{$t('import.failed', { n: report.failed })}</p>
      <ul class="list">
        {#each report.failures as f (f.source)}
          <li><span class="path">{f.source}</span> <span class="muted">{f.error}</span></li>
        {/each}
      </ul>
    {/if}
    {#if report.plans.length}
      <h3 class="sec">{$t('import.openPlans')}</h3>
      {#each report.plans as p (p.id)}
        <button class="mini" onclick={() => openPlan(p.id)}>
          <span class="dot" style="background:{$projectColor(p.project)}"></span>
          <span class="grow">
            <b>{p.title}</b>
            <span class="meta">{projectLabel(p.project)} · {p.id}</span>
          </span>
          <span class="chev" aria-hidden="true">›</span>
        </button>
      {/each}
    {/if}
  </div>
{/if}

<p class="hint">
  {$t('import.hint')}
  <code>{$t('import.hintCmd')}</code>
</p>

<style>
  .kind {
    color: var(--mute);
    font-size: 12.5px;
  }
  .lead {
    margin-top: 0;
  }
  .pickers {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin: 14px 0 4px;
  }
  .pick {
    position: relative;
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 13px;
    background: var(--panel);
    cursor: pointer;
  }
  .pick:hover,
  .pick:focus-within {
    border-color: var(--accent);
  }
  /* The real input covers the label so it stays clickable and focusable. */
  .pick input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
  }
  .preview .count {
    font-weight: 600;
    margin-bottom: 6px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 220px;
    overflow: auto;
    font-size: 12.5px;
  }
  .list li {
    display: flex;
    gap: 10px;
    padding: 3px 0;
    border-top: 1px solid var(--line);
  }
  .list li:first-child {
    border-top: 0;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .size {
    color: var(--faint);
    flex: none;
  }
  .small {
    font-size: 12px;
    margin-top: 4px;
  }
  .note {
    margin-top: 8px;
    font-size: 12.5px;
    color: var(--mute);
    background: var(--bg);
    border-radius: 6px;
    padding: 6px 10px;
  }
  .problem {
    color: var(--warn);
    background: var(--warn-soft);
    border-radius: 6px;
    padding: 6px 10px;
    margin: 8px 0 0;
    font-size: 13px;
  }
  .field {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin: 14px 0;
  }
  .field label {
    font-weight: 600;
    font-size: 13px;
  }
  select,
  .name {
    font: inherit;
    font-size: 13px;
    color: var(--ink);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 5px 10px;
  }
  select:focus,
  .name:focus {
    border-color: var(--accent);
    outline: none;
  }
  .name {
    flex: 1;
    min-width: 160px;
  }
  .go {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
    padding: 7px 18px;
  }
  .go:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .result {
    margin-top: 16px;
  }
  .ok {
    color: var(--ok);
    background: var(--ok-soft);
    border-radius: 8px;
    padding: 10px 12px;
    margin: 0;
    font-weight: 600;
  }
  .grow b {
    display: block;
    font-weight: 600;
  }
  .meta {
    color: var(--mute);
    font-size: 12px;
  }
  .hint {
    margin-top: 26px;
    padding-top: 12px;
    border-top: 1px dashed var(--line);
    color: var(--mute);
    font-size: 12.5px;
  }
  .hint code {
    display: inline-block;
    margin-top: 4px;
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 1px 6px;
    color: var(--ink);
  }
</style>
