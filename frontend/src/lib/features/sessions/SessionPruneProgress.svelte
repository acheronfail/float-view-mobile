<script lang="ts">
  import type { SessionOperationState } from '$lib/backend/contract';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';

  let {
    operation,
    oncancel,
  }: {
    operation: SessionOperationState | null | undefined;
    oncancel: (operationId: string) => void | Promise<void>;
  } = $props();

  const active = $derived(
    operation?.kind === 'prune' && ['queued', 'running', 'reclaiming', 'cancelling'].includes(operation.status)
  );
  function cancel() {
    if (active && operation && operation.status !== 'cancelling') void oncancel(operation.operationId);
  }
</script>

{#if active && operation}
  <div class="flex items-center justify-between gap-3 border-b border-outline bg-surface px-4 py-2 text-sm">
    <p role="status">
      {operation.status === 'reclaiming'
        ? 'Reclaiming space…'
        : operation.status === 'cancelling'
          ? 'Cancelling prune…'
          : 'Pruning sessions…'}
    </p>
    <button
      type="button"
      class="touch-manipulation rounded-lg border border-outline px-3 py-2"
      disabled={operation.status === 'cancelling'}
      onpointerdown={(event) => activateFromPointer(event, cancel)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, cancel)}>Cancel</button
    >
  </div>
{/if}
