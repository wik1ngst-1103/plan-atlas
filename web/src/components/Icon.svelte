<script lang="ts" module>
  /** Line icons, 24×24 grid, drawn with the current text colour. */
  const PATHS = {
    file: 'M14 2.5H6.5a2 2 0 0 0-2 2v15a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V8zM14 2.5V8h5.5',
    clock: 'M12 7v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z',
    calendar:
      'M8 2.5v4M16 2.5v4M3.5 10h17M5.5 4.5h13a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-12a2 2 0 0 1 2-2z',
    list: 'M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01',
    link: 'M10 13.5a4.5 4.5 0 0 0 6.8.5l2.9-2.9a4.5 4.5 0 0 0-6.4-6.4l-1.6 1.6M14 10.5a4.5 4.5 0 0 0-6.8-.5l-2.9 2.9a4.5 4.5 0 0 0 6.4 6.4l1.6-1.6',
    hash: 'M5 9h15M4 15h15M10 3.5 8 20.5M16 3.5l-2 17',
    up: 'M12 19.5v-15M5.5 11 12 4.5l6.5 6.5',
    down: 'm6 9 6 6 6-6',
    check: 'M20 6.5 9.5 17 4 11.5',
    folder:
      'M3.5 7.5v10a2 2 0 0 0 2 2h13a2 2 0 0 0 2-2v-8a2 2 0 0 0-2-2h-6.5l-2-3h-4.5a2 2 0 0 0-2 2z',
    tasks: 'M9 11.5l2.5 2.5L20 5.5M20 12v6.5a2 2 0 0 1-2 2H5.5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h9',
  } as const;
  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name]} />
</svg>

<style>
  .icon {
    flex: none;
    display: inline-block;
    vertical-align: -0.15em;
  }
</style>
