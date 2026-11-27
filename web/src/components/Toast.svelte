<script lang="ts">
  import { toastMsg } from '../lib/toast';

  /** Short confirmation at the bottom centre ("Moved “X” to Done"). */
  let last = $state('');
  let error = $state(false);
  $effect(() => {
    // Keep the text while sliding out.
    if ($toastMsg) {
      last = $toastMsg.text;
      error = $toastMsg.error;
    }
  });
</script>

<div class="toast" class:on={$toastMsg} class:error role="status" aria-live="polite">
  {$toastMsg ? $toastMsg.text : last}
</div>

<style>
  .toast {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translate(-50%, 80px);
    background: var(--ink);
    color: var(--bg);
    padding: 9px 16px;
    border-radius: 8px;
    font-size: 13px;
    transition: transform 0.2s;
    z-index: 20;
    max-width: calc(100vw - 32px);
    pointer-events: none;
  }
  .toast.on {
    transform: translate(-50%, 0);
  }
  .toast.error {
    background: var(--warn);
  }
</style>
