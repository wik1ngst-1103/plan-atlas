import { writable } from 'svelte/store';

export type ToastMsg = { id: number; text: string; error: boolean };

const SHOW_MS = 2200;

/** The message on screen, or null. */
export const toastMsg = writable<ToastMsg | null>(null);

let n = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

/** Show a short message at the bottom of the screen for 2.2 s. */
export function toast(text: string, opts: { error?: boolean } = {}) {
  const id = ++n;
  toastMsg.set({ id, text, error: !!opts.error });
  clearTimeout(timer);
  timer = setTimeout(() => toastMsg.update((m) => (m?.id === id ? null : m)), SHOW_MS);
}
