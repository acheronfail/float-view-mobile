<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import type { ConnectionContextEvent, ContextFirmware, SessionMetadata } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  let { metadata }: { metadata: SessionMetadata } = $props();
  let rows = $state<ConnectionContextEvent[]>([]);
  let next = $state<string | null>(null);
  let cursor = $state<string | null>(null);
  let busy = $state(false);
  let error = $state('');
  let retry = $state(0);
  // One page and one request owner; changing snapshots disposes obsolete reads.
  $effect(() => {
    const current = metadata;
    const after = cursor;
    void retry;
    const controller = backendState.createSessionReviewController(current);
    let cancelled = false;
    rows = [];
    next = null;
    error = '';
    const shouldLoad = current.connectionContextCount > 0;
    busy = shouldLoad;
    if (shouldLoad)
      void controller
        .loadConnectionContexts({
          kind: 'connectionContexts',
          limit: 64,
          cursor: after,
          range: { startElapsedMs: 0, endElapsedMs: current.snapshot.committedElapsedMs },
        })
        .then((result) => {
          if (!cancelled) {
            rows = result.contexts;
            next = result.nextCursor;
          }
        })
        .catch(() => {
          if (!cancelled) error = 'Could not load connection history.';
        })
        .finally(() => {
          if (!cancelled) busy = false;
        });
    return () => {
      cancelled = true;
      controller.dispose();
    };
  });
  $effect(() => {
    void metadata.summary.sessionId;
    void metadata.snapshot.historyRevision;
    cursor = null;
  });
  function firmware(value: ContextFirmware) {
    return `${value.major}.${String(value.minor).padStart(2, '0')}${value.testVersion ? ` (test ${value.testVersion})` : ''}`;
  }
  function elapsed(ms: number) {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  }
</script>

<section aria-label="Connection history" class="my-4 space-y-2 rounded-lg border border-outline bg-elevated p-3">
  <h2 class="font-semibold">Connection history</h2>
  {#if metadata.connectionContextCount === 0}
    <p class="text-sm text-muted">Not recorded</p>
  {:else if busy}
    <p role="status" class="text-sm">Loading connection history…</p>
  {:else if error}
    <p role="alert">{error}</p>
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
        })}>Retry connections</button
    >
  {:else}
    {#if !cursor && rows[0]?.elapsedMs > 0}<p class="text-sm text-muted">
        Earlier connection context: Not recorded
      </p>{/if}
    <ol class="space-y-3">
      {#each rows as event, index (event.sequence)}
        <li class="text-sm">
          <p class="font-medium">
            {!cursor && index === 0 ? 'First recorded connection' : 'Reconnect'} · {elapsed(event.elapsedMs)}
          </p>
          <p>Refloat {event.context.refloatVersion} · Controller VESC {firmware(event.context.controller)}</p>
          {#if event.context.controller.hardwareName}<p>{event.context.controller.hardwareName}</p>{/if}
          {#if event.context.connection.kind === 'direct'}
            <p>Direct connection</p>
          {:else}
            <p>
              Via {event.context.transport.firmwareName ?? event.context.transport.hardwareName ?? 'Bluetooth peer'} → CAN
              {event.context.connection.canId}
            </p>
            <p>Peer VESC {firmware(event.context.transport)}</p>
          {/if}
        </li>
      {/each}
    </ol>
    <div class="flex gap-2">
      {#if cursor}<button
          type="button"
          class="min-h-11 touch-manipulation rounded border border-outline px-3 text-sm"
          onpointerdown={(e) =>
            activateFromPointer(e, () => {
              cursor = null;
            })}
          onclick={(e) =>
            activateFromKeyboardOrAssistiveClick(e, () => {
              cursor = null;
            })}>First connections</button
        >{/if}
      {#if next}<button
          type="button"
          class="min-h-11 touch-manipulation rounded border border-outline px-3 text-sm"
          onpointerdown={(e) =>
            activateFromPointer(e, () => {
              cursor = next;
            })}
          onclick={(e) =>
            activateFromKeyboardOrAssistiveClick(e, () => {
              cursor = next;
            })}>Next connections</button
        >{/if}
    </div>
  {/if}
</section>
