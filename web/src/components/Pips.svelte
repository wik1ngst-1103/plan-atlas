<script lang="ts">
  /** Five little bars, `n` of them filled: a visual for difficulty 1–5. */
  let { n }: { n: number } = $props();
</script>

<span class="pips" aria-hidden="true">
  {#each [1, 2, 3, 4, 5] as i (i)}<i class:f={i <= n}></i>{/each}
</span>

<style>
  .pips i {
    display: inline-block;
    width: 6px;
    height: 10px;
    border-radius: 2px;
    background: var(--line);
    margin-right: 2px;
    vertical-align: middle;
  }
  .pips i.f {
    background: var(--warn);
  }
</style>
