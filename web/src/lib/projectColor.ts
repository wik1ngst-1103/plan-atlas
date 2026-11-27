import { derived, writable } from 'svelte/store';

const COLORS = ['#4c6ef5', '#12b886', '#f59f00', '#e64980', '#7950f2', '#15aabf'];

/** Sorted, unique names of all known projects (set from `/api/projects`). */
export const knownProjects = writable<string[]>([]);

export function setKnownProjects(names: string[]) {
  knownProjects.set([...new Set(names)].sort());
}

/**
 * Colour by the project's index in the sorted list of known projects, so
 * colours are distinct for up to `COLORS.length` projects; names not in the
 * list fall back to a hash.
 */
export function colorFor(name: string, known: string[]): string {
  const i = known.indexOf(name);
  if (i >= 0) return COLORS[i % COLORS.length];
  return COLORS[[...name].reduce((h, c) => h + c.charCodeAt(0), 0) % COLORS.length];
}

/** `$projectColor(name)`: the same colour everywhere (side nav, cards, map legend). */
export const projectColor = derived(
  knownProjects,
  (known) => (name: string) => colorFor(name, known),
);
