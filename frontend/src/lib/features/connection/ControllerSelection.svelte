<script lang="ts">
  import type { ActiveConnection } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import BottomSheet from '$lib/ui/BottomSheet.svelte';
  import { sortedControllerCandidates } from './live';

  let { connection, ondisconnect }: { connection: ActiveConnection; ondisconnect: () => Promise<void> } = $props();
  let pendingCanId = $state<number | null>(null);
  const candidates = $derived(sortedControllerCandidates(connection.controllerCandidates));

  async function select(canId: number) {
    if (pendingCanId !== null || backendState.busy) return;
    pendingCanId = canId;
    const accepted = await backendState.selectController(connection.deviceId, canId);
    if (!accepted) pendingCanId = null;
  }
</script>

<BottomSheet
  title={`Choose a controller (${candidates.length} found)`}
  titleId="controller-selection-title"
  closeLabel="Disconnect"
  onclose={ondisconnect}
>
  <p class="mb-4 text-sm text-muted">
    More than one VESC motor controller responded. Choose the controller fitted to this board.
  </p>
  <ul class="grid gap-3">
    {#each candidates as candidate (candidate.canId)}
      <li>
        <button
          type="button"
          class="w-full touch-manipulation rounded-xl border border-outline bg-elevated p-4 text-left disabled:cursor-not-allowed disabled:opacity-50"
          disabled={!candidate.vescSupported || pendingCanId !== null || backendState.busy}
          onpointerdown={(event) => activateFromPointer(event, () => select(candidate.canId))}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => select(candidate.canId))}
        >
          <span class="flex items-center justify-between gap-3">
            <strong>CAN {candidate.canId}</strong>
            <span class={candidate.vescSupported ? 'text-success' : 'text-warning'}>
              {pendingCanId === candidate.canId ? 'Connecting…' : candidate.vescSupported ? 'Supported' : 'Unsupported'}
            </span>
          </span>
          <span class="mt-1 block text-sm text-muted">
            {candidate.hardwareName ?? 'Unknown hardware'} ·
            {candidate.firmwareName ?? 'Unknown firmware'} · VESC {candidate.vescVersion}
          </span>
          <code class="mt-1 block text-xs text-muted">{candidate.uuid ?? 'UUID unavailable'}</code>
        </button>
      </li>
    {/each}
  </ul>
  {#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</BottomSheet>
