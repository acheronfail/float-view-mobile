<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import SessionHistory from './SessionHistory.svelte';
  import SessionEvents from './SessionEvents.svelte';

  let { sessionId }: { sessionId: string } = $props();
  let deleteConfirmationOpen = $state(false);
  let deleting = $state(false);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

  onMount(() => {
    let active = true;
    void backendState.openSession(sessionId).then((opened) => {
      if (active && !opened) void goto('#/sessions', { replaceState: true });
    });
    return () => {
      active = false;
      backendState.closeSession();
    };
  });

  function requestDeleteSession() {
    if (backendState.selectedSession) deleteConfirmationOpen = true;
  }

  function cancelDeleteSession() {
    deleteConfirmationOpen = false;
  }

  async function deleteSession() {
    if (deleting) return;
    deleting = true;
    const deleted = await backendState.deleteSelectedSession();
    deleting = false;
    deleteConfirmationOpen = false;
    if (deleted) await goto('#/sessions', { replaceState: true });
  }
</script>

<section aria-labelledby="session-heading">
  <a class="text-accent" href="#/sessions">← Sessions</a>
  {#if backendState.selectedSession}
    <h2 id="session-heading" class="mt-3 text-xl font-medium">Review</h2>
    <p class="mt-2 text-lg">{backendState.selectedSession.displayName}</p>
    <div class="mt-4 grid grid-cols-2 gap-3">
      <div class="rounded-lg border border-outline bg-elevated p-3">
        <p class="text-sm text-muted">Samples</p>
        <p class="text-xl">{backendState.selectedSession.sampleCount}</p>
      </div>
      <div class="rounded-lg border border-outline bg-elevated p-3">
        <p class="text-sm text-muted">Distance</p>
        <p class="text-xl">
          {formatDistanceM(backendState.selectedSession.distanceM, units)}
        </p>
      </div>
    </div>

    {#if backendState.sessionMetadata}
      <SessionHistory metadata={backendState.sessionMetadata} />
    {/if}

    {#if backendState.sessionMetadata}<SessionEvents metadata={backendState.sessionMetadata} />{/if}

    <div class="mt-5 rounded-lg border border-outline p-4">
      <h3 class="font-medium">Session file</h3>
      <p class="mt-1 text-sm text-muted">Exports open in the native platform save or share surface.</p>
      <div class="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          class="touch-manipulation rounded-md bg-elevated px-3 py-2 disabled:opacity-50"
          disabled={backendState.busy || backendState.selectedSession.state === 'active'}
          onpointerdown={(event) => activateFromPointer(event, () => backendState.exportSession('float-view-archive'))}
          onclick={(event) =>
            activateFromKeyboardOrAssistiveClick(event, () => backendState.exportSession('float-view-archive'))}
          >Save/share archive</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-md bg-elevated px-3 py-2 disabled:opacity-50"
          disabled={backendState.busy || backendState.selectedSession.state === 'active'}
          onpointerdown={(event) => activateFromPointer(event, () => backendState.exportSession('csv'))}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => backendState.exportSession('csv'))}
          >Save/share CSV</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-md bg-danger px-3 py-2 text-on-danger disabled:opacity-50"
          disabled={backendState.busy || backendState.selectedSession.state === 'active'}
          onpointerdown={(event) => activateFromPointer(event, requestDeleteSession)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, requestDeleteSession)}>Delete session</button
        >
      </div>
      {#if backendState.exportStatus}<p class="mt-3 text-sm text-success">
          {backendState.exportStatus}
        </p>{/if}
    </div>
  {:else}
    <p class="mt-3 text-muted">Loading session…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>

<ConfirmDialog
  open={deleteConfirmationOpen}
  titleId="delete-session-confirmation-title"
  title="Delete session?"
  description={`Permanently delete this ${backendState.selectedSession?.sampleCount ?? 0}-sample session? This cannot be undone.`}
  confirmLabel="Delete"
  destructive
  busy={deleting}
  onconfirm={deleteSession}
  oncancel={cancelDeleteSession}
/>
