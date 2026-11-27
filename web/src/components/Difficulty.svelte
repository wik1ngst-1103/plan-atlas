<script lang="ts">
  import type { Task } from '../lib/generated/Task';
  import { t } from '../lib/i18n';
  import { MODEL_NAME, taskModel } from '../lib/plans';
  import Pips from './Pips.svelte';

  /** "▮▮▮▯▯ Medium (3/5) · Claude uses Sonnet"; `short` drops the model part. */
  let { task, short = false }: { task: Task; short?: boolean } = $props();
  const model = $derived(taskModel(task));
  const c = $derived(task.complexity);
</script>

<span
  class="diff"
  title={model
    ? $t('diff.modelHint', { model: MODEL_NAME[model], desc: $t(`model.${model}`) })
    : ''}
>
  {#if c}
    <Pips n={c} />
    {$t('diff.level', { level: $t(`diff.${c}`), n: c })}
  {:else}
    {$t('diff.none')}
  {/if}
  {#if model && !short}
    · {$t('diff.uses', { model: MODEL_NAME[model] })}
  {/if}
</span>

<style>
  .diff {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex-wrap: wrap;
  }
</style>
