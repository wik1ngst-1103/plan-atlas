<script lang="ts">
  import type { TaskStatus } from '../lib/generated/TaskStatus';
  import { t } from '../lib/i18n';

  const ICON: Record<TaskStatus, string> = { todo: '○', in_progress: '◐', review: '◑', done: '●' };
  let { status }: { status: TaskStatus } = $props();
</script>

<span class="status st-{status}">{ICON[status]} {$t(`status.${status}`)}</span>
