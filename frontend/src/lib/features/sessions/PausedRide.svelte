<script lang="ts">
  import { onMount } from 'svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import type { PausedRide } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  let { ride }: { ride: PausedRide } = $props();
  let now = $state(Date.now());
  let pending = $state(false);
  let confirmationOpen = $state(false);
  let endButton = $state<HTMLButtonElement>();
  function requestEnd() {
    endButton?.focus();
    confirmationOpen = true;
    error = '';
  }
  let error = $state('');
  const minutes = $derived(Math.max(0, Math.ceil((Date.parse(ride.resumeUntil) - now) / 60_000)));
  onMount(() => {
    const timer = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(timer);
  });
  async function end() {
    if (pending) return;
    pending = true;
    error = '';
    if (!(await backendState.endRide(ride.sessionId))) error = 'Could not end the ride. Try again.';
    else confirmationOpen = false;
    pending = false;
  }
</script>

<aside aria-label="Paused ride" class="space-y-2 rounded-lg border border-outline bg-surface p-3">
  <p class="font-medium">{ride.displayName} · Ride paused</p>
  <p role="status" class="text-sm text-muted">
    {minutes > 0
      ? `Reconnect within ${minutes} ${minutes === 1 ? 'minute' : 'minutes'} to resume this ride.`
      : 'Resume window ended.'}
  </p>
  <button
    type="button"
    bind:this={endButton}
    class="min-h-11 touch-manipulation rounded border border-outline px-3 text-sm"
    disabled={pending || backendState.busy}
    onpointerdown={(event) => activateFromPointer(event, requestEnd)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, requestEnd)}>End ride</button
  >
  {#if error}<p role="alert" class="text-sm text-danger">{error}</p>{/if}
</aside>

<ConfirmDialog
  open={confirmationOpen}
  titleId="paused-end-ride-title"
  title="End ride?"
  description="This will finish this ride. Reconnecting will start a new ride."
  confirmLabel="End ride"
  appearance="destructive"
  busy={pending}
  onconfirm={end}
  oncancel={() => {
    confirmationOpen = false;
  }}
>
  {#if error}<p role="alert" class="text-sm text-danger">{error}</p>{/if}
</ConfirmDialog>
