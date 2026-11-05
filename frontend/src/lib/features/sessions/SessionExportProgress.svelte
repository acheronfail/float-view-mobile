<script lang="ts">
  import { getContext, tick } from 'svelte';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import FullScreenOverlay from '$lib/ui/FullScreenOverlay.svelte';
  const header = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  $effect(() => {
    if (!backendState.boardExport) return;
    return () => {
      void tick().then(() => header.focusMenu());
    };
  });
  const progress = $derived(backendState.boardExport);
</script>

{#if progress}
  <FullScreenOverlay
    title="Exporting sessions"
    titleId="session-export-progress-title"
    closeLabel={progress.cancelRequested ? 'Cancelling…' : 'Cancel'}
    closeDisabled={progress.cancelRequested || (progress.phase === 'ready' && backendState.platform !== 'web')}
    onclose={() => backendState.cancelBoardExport()}
  >
    <div class="space-y-5 py-6">
      <p role="status" aria-live="polite">
        {#if progress.cancelRequested || progress.phase === 'cancelling'}Cancelling export…
        {:else if progress.phase === 'downloading'}Downloading ZIP…
        {:else if progress.phase === 'ready'}Opening save or share…
        {:else if progress.totalSessions}Exported {progress.completedSessions} of {progress.totalSessions} sessions
        {:else}Preparing export…{/if}
      </p>
      {#if progress.totalSessions}
        <progress
          class="h-3 w-full accent-accent"
          aria-label="Sessions exported"
          value={progress.completedSessions}
          max={progress.totalSessions}
        ></progress>
      {:else}
        <progress class="h-3 w-full accent-accent" aria-label="Preparing export"></progress>
      {/if}
      <p class="text-sm text-muted">Keep the app open while the ZIP is prepared. Cancelling preserves your sessions.</p>
    </div>
  </FullScreenOverlay>
{/if}
