import { writable } from 'svelte/store';

/** Topic the knowledge map should focus on (set by "View on map"; the map page reads it). */
export const mapTopic = writable<string | null>(null);
