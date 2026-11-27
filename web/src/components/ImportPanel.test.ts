import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { drawer } from '../lib/drawer';
import { setLocale } from '../lib/i18n';
import { buildImportForm, limitProblem, splitSelection } from '../lib/importFiles';
import ImportPanel from './ImportPanel.svelte';

/** A File as a folder pick yields it (`webkitRelativePath` set). */
function file(path: string, text = '# Plan\n'): File {
  const f = new File([text], path.split('/').at(-1)!, { type: 'text/markdown' });
  if (path.includes('/')) Object.defineProperty(f, 'webkitRelativePath', { value: path });
  return f;
}

async function pick(input: HTMLInputElement, files: File[]) {
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  await fireEvent.change(input);
}

describe('import file selection', () => {
  it('keeps .md files and leaves out the rest', () => {
    const { md, ignored } = splitSelection([
      file('plans/a.md'),
      file('plans/sub/B.MD'),
      file('plans/notes.txt'),
      file('plans/.hidden.md'),
      file('plans/.git/c.md'),
      file('plans/node_modules/x/d.md'),
      file('loose.md'),
    ]);
    expect(md.map((f) => f.webkitRelativePath || f.name)).toEqual([
      'plans/a.md',
      'plans/sub/B.MD',
      'loose.md',
    ]);
    expect(ignored).toBe(4);
  });

  it('flags files over the upload limits', () => {
    const big = new File([new Uint8Array(2 * 1024 * 1024 + 1)], 'big.md');
    expect(limitProblem([big])).toEqual({ kind: 'tooBig', name: 'big.md' });
    expect(limitProblem([file('ok.md')])).toBeNull();
  });

  it('builds the form with relative names and the project', () => {
    const form = buildImportForm([file('plans/sub/a.md'), file('b.md')], ' nsvideo ');
    const parts = form.getAll('files') as File[];
    expect(parts.map((f) => f.name)).toEqual(['plans/sub/a.md', 'b.md']);
    expect(form.get('project')).toBe('nsvideo');
    expect(buildImportForm([file('b.md')], '').has('project')).toBe(false);
  });
});

describe('ImportPanel', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    setLocale('en');
    fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
      if (url === '/api/projects')
        return new Response(
          JSON.stringify([{ name: 'nsvideo', plan_count: 1, task_count: 0, done_count: 0 }]),
        );
      if (url === '/api/import' && init?.method === 'POST')
        return new Response(
          JSON.stringify({
            imported: 2,
            skipped: 1,
            unsorted: 0,
            failed: 0,
            plans: [
              { id: 'PLAN-nsvideo-002', project: 'nsvideo', title: 'Alpha' },
              { id: 'PLAN-nsvideo-003', project: 'nsvideo', title: 'Beta' },
            ],
            failures: [],
          }),
        );
      return new Response('[]');
    });
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    drawer.close();
  });

  it('previews a folder, uploads it as FormData and reports the result', async () => {
    render(ImportPanel);
    const submit = screen.getByRole('button', { name: 'Import' }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);

    const folder = screen.getByLabelText('Choose a folder') as HTMLInputElement;
    expect(folder.hasAttribute('webkitdirectory')).toBe(true);
    await pick(folder, [file('p/a.md'), file('p/b.md'), file('p/c.md'), file('p/x.png')]);
    expect(screen.getByText(/3 plans selected/)).toBeTruthy();
    expect(screen.getByText(/1 other file was left out/)).toBeTruthy();
    expect(screen.getByText('p/a.md')).toBeTruthy();

    const select = (await screen.findByRole('combobox', { name: 'Project' })) as HTMLSelectElement;
    await screen.findByRole('option', { name: 'nsvideo' });
    await fireEvent.change(select, { target: { value: 'nsvideo' } });

    expect(submit.disabled).toBe(false);
    await fireEvent.click(submit);

    const call = fetchMock.mock.calls.find(([u]) => u === '/api/import')!;
    const init = call[1] as RequestInit;
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>)['X-Plan-Atlas']).toBe('1');
    expect(init.body).toBeInstanceOf(FormData);
    const form = init.body as FormData;
    expect((form.getAll('files') as File[]).map((f) => f.name)).toEqual([
      'p/a.md',
      'p/b.md',
      'p/c.md',
    ]);
    expect(form.get('project')).toBe('nsvideo');

    expect(
      await screen.findByText('Imported 2 plans into nsvideo. 1 skipped (already imported).'),
    ).toBeTruthy();
    expect(screen.getByRole('button', { name: /Alpha/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Beta/ })).toBeTruthy();
  });

  it('accepts a new project name', async () => {
    render(ImportPanel);
    await pick(screen.getByLabelText('Choose files') as HTMLInputElement, [file('one.md')]);
    const select = (await screen.findByRole('combobox', { name: 'Project' })) as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: '__new' } });
    const name = screen.getByLabelText('New project name') as HTMLInputElement;
    await fireEvent.input(name, { target: { value: 'Side Quest' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Import' }));
    const init = fetchMock.mock.calls.find(([u]) => u === '/api/import')![1] as RequestInit;
    expect((init.body as FormData).get('project')).toBe('Side Quest');
  });
});
