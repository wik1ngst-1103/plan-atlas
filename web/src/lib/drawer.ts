import { writable } from 'svelte/store';

export type DrawerItem =
  | { type: 'task'; plan: string; task: string }
  | { type: 'topic'; tag: string }
  | { type: 'plan'; plan: string }
  | { type: 'activity' }
  | { type: 'import' };

export type DrawerStack = DrawerItem[];

export const drawerOpen = (
  stack: DrawerStack,
  item: DrawerItem,
  opts: { push?: boolean } = {},
): DrawerStack => (opts.push ? [...stack, item] : [item]);
export const drawerBack = (stack: DrawerStack): DrawerStack => stack.slice(0, -1);
export const drawerClose = (): DrawerStack => [];

const store = writable<DrawerStack>([]);

/** Drawer stack; the top item is the one shown. */
export const drawer = {
  subscribe: store.subscribe,
  open: (item: DrawerItem, opts?: { push?: boolean }) =>
    store.update((s) => drawerOpen(s, item, opts)),
  back: () => store.update(drawerBack),
  close: () => store.set(drawerClose()),
};

/** "Show technical details" in the task panel (kept while the app is open). */
export const showRaw = writable(false);
