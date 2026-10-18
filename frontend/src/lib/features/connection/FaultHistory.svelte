<script lang="ts">
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import { untrack } from 'svelte';
  import { loadFirmwareFaults } from '$lib/features/faults/session-faults';
  import type { RideFault } from '$lib/features/faults/faults';
  import FullScreenOverlay from '$lib/ui/FullScreenOverlay.svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import FaultList from '$lib/features/faults/FaultList.svelte';
  import { firmwareFaultWarning } from './live-status';
  let { onclose }: { onclose: () => void } = $props();
  const connection = $derived(backendState.snapshot?.activeConnection);
  const sample = $derived(connection?.latestSample);
  let recorded = $state<RideFault[] | null>(null);
  let error = $state('');
  let retry = $state(0);
  let recordedSession: string | undefined;
  const sessionId = $derived(connection?.activeSession?.sessionId);
  const version = $derived(connection?.activeSession?.eventCount ?? sample?.faultHistory?.at(-1)?.observedElapsedMs);
  $effect(() => {
    void retry;
    const id = sessionId;
    void version;
    if (recordedSession !== id) {
      recorded = null;
      recordedSession = id;
    }
    if (!id) {
      recorded = null;
      return;
    }
    let active = true;
    let controller: ReturnType<typeof backendState.createSessionReviewController> | undefined;
    error = '';
    untrack(() => {
      void backendState
        .readSessionMetadata(id)
        .then(async (metadata) => {
          if (!active) return;
          controller = backendState.createSessionReviewController(metadata);
          const faults = await loadFirmwareFaults(controller, metadata);
          if (active) recorded = faults;
        })
        .catch((reason) => {
          if (active) error = reason instanceof Error ? reason.message : 'History could not be loaded.';
        });
    });
    return () => {
      active = false;
      controller?.dispose();
    };
  });
  const faults = $derived(recorded ?? (sessionId ? [] : (sample?.faultHistory ?? [])));
  const warning = $derived(firmwareFaultWarning(sample));
</script>

<FullScreenOverlay title="Fault history" titleId="fault-history-title" {onclose}>
  <div class="mx-auto w-full max-w-2xl space-y-4 p-4">
    {#if warning}
      <div role="status" class="rounded-lg border border-danger/40 bg-danger/5 p-3 text-sm">
        <p class="font-semibold text-danger">
          {sample?.firmwareFault ? 'Controller fault active' : 'Fatal error latched'}
        </p>
        <p class="mt-1">Stop riding and investigate the cause before clearing or power cycling.</p>
      </div>
    {/if}
    <p class="text-sm text-muted">
      Recent faults from this ride. Times are approximate; older faults may have no ride time.
    </p>
    {#if error}<p role="alert" class="text-sm text-danger">{error}</p>
      <button
        type="button"
        class="min-h-11 touch-manipulation rounded border border-outline px-3 text-sm"
        onpointerdown={(e) =>
          activateFromPointer(e, () => {
            retry++;
          })}
        onclick={(e) =>
          activateFromKeyboardOrAssistiveClick(e, () => {
            retry++;
          })}>Retry</button
      >{/if}
    {#if faults.length}
      <FaultList faults={[...faults].reverse()} />
      {#if !sessionId}<p class="text-xs text-muted">Guest history keeps the 20 most recent faults.</p>{/if}
    {:else if sessionId && recorded === null && !error}
      <p class="py-6 text-center text-muted" role="status">Loading history…</p>
    {:else if !error}
      <p class="py-6 text-center text-muted">No faults recorded.</p>
    {/if}
  </div>
</FullScreenOverlay>
