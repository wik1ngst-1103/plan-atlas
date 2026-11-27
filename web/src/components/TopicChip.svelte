<script lang="ts">
  import { drawer } from '../lib/drawer';

  /** `tag` like "tech/jwt"; `label` is the localized name from /api/topics when known. */
  let { tag, label }: { tag: string; label?: string } = $props();
  const [group, name] = $derived(tag.split('/'));
</script>

<button class="chip g-{group}" onclick={() => drawer.open({ type: 'topic', tag }, { push: true })}>
  {label ?? name}
</button>
