/** Picking `.md` plans in the browser for `POST /api/import` (limits mirror the server). */

export const MAX_FILES = 200;
export const MAX_FILE_BYTES = 2 * 1024 * 1024;
export const MAX_TOTAL_BYTES = 20 * 1024 * 1024;

/** Folders the command-line import also skips. */
const SKIP_DIRS = new Set(['node_modules', 'target']);

/** Path inside the picked folder, or the bare file name for a file pick. */
export const relPath = (f: File): string => f.webkitRelativePath || f.name;

/**
 * Keep `.md` files, leaving out anything inside hidden, `node_modules` or `target`
 * folders and hidden files (as `plan-atlas import <folder>` does). Duplicate paths
 * (the same file picked twice) are kept once.
 */
export function splitSelection(files: Iterable<File>): { md: File[]; ignored: number } {
  const md: File[] = [];
  const seen = new Set<string>();
  let ignored = 0;
  for (const f of files) {
    const path = relPath(f);
    const parts = path.split('/');
    const name = parts.at(-1) ?? '';
    // A folder pick's first part is the picked folder itself: never skip it.
    const dirs = parts.slice(1, -1);
    const skipped =
      !/\.md$/i.test(name) ||
      name.startsWith('.') ||
      dirs.some((d) => d.startsWith('.') || SKIP_DIRS.has(d));
    if (skipped) ignored++;
    else if (!seen.has(path)) {
      seen.add(path);
      md.push(f);
    }
  }
  return { md, ignored };
}

export type LimitProblem =
  { kind: 'tooMany'; max: number } | { kind: 'tooBig'; name: string } | { kind: 'tooBigTotal' };

/** The first upload limit the selection breaks, if any. */
export function limitProblem(files: File[]): LimitProblem | null {
  if (files.length > MAX_FILES) return { kind: 'tooMany', max: MAX_FILES };
  const big = files.find((f) => f.size > MAX_FILE_BYTES);
  if (big) return { kind: 'tooBig', name: relPath(big) };
  if (totalSize(files) > MAX_TOTAL_BYTES) return { kind: 'tooBigTotal' };
  return null;
}

export const totalSize = (files: File[]) => files.reduce((n, f) => n + f.size, 0);

/** `512 B`, `12 KB`, `1.4 MB`. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** The request body: every file as `files` (named by its relative path), plus `project`. */
export function buildImportForm(files: File[], project: string): FormData {
  const form = new FormData();
  for (const f of files) form.append('files', f, relPath(f));
  const p = project.trim();
  if (p) form.append('project', p);
  return form;
}
