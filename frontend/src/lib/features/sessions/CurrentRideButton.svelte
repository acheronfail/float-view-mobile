<script lang="ts">
  import { goto } from '$app/navigation';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';

  const connection = $derived(backendState.snapshot?.activeConnection);
  const session = $derived(connection?.trackingMode === 'saved' ? connection.activeSession : null);
  function open() {
    if (session) void goto(`#/sessions/${encodeURIComponent(session.sessionId)}`);
  }
</script>

{#if session}
  <button
    type="button"
    class="flex min-h-11 w-full shrink-0 touch-manipulation items-center justify-between rounded-lg border border-outline bg-surface px-3 py-2 text-left"
    onpointerdown={(event) => activateFromPointer(event, open)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, open)}
  >
    <span>Current ride</span>
    <span class="text-xs text-muted">{connection?.state === 'reconnecting' ? 'Reconnecting' : 'Recording'} →</span>
  </button>
{/if}
