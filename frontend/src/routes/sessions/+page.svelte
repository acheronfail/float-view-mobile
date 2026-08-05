<script lang="ts">
  import { getContext, onDestroy, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import { reportFrontendPerformance } from '$lib/backend/performance-log';
  import BoardList from '$lib/features/boards/BoardList.svelte';
  import PruneDialog from '$lib/features/sessions/PruneDialog.svelte';

  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  let importInput = $state<HTMLInputElement>();
  let pruneOpen = $state(false);

  $effect(() => {
    appHeader.set('/sessions', {
      menuLabel: 'Session actions',
      menu: [
        {
          label: 'Import session',
          disabled: backendState.busy || backendState.sessionImporting,
          onactivate: (closeMenu: () => void) => chooseImport(closeMenu),
        },
        {
          label: 'Prune all',
          appearance: 'warning',
          disabled: backendState.busy,
          onactivate: (closeMenu: () => void) => openPrune(closeMenu),
        },
      ],
    });
  });
  onDestroy(() => appHeader.clear('/sessions'));

  function chooseImport(closeMenu: () => void) {
    closeMenu();
    importInput?.click();
  }

  function openPrune(closeMenu: () => void) {
    closeMenu();
    pruneOpen = true;
  }

  async function closePrune() {
    pruneOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function importSession(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    reportFrontendPerformance('session-import-file', {
      hasFile: Boolean(file),
      bytes: file?.size ?? 0,
      type: file?.type || 'unknown',
    });
    input.value = '';
    if (!file) return;
    const sessionId = await backendState.importSession(file);
    if (sessionId) await goto(await backendState.sessionHref(sessionId));
  }
</script>

<section aria-labelledby="app-title" class="space-y-4">
  <input
    bind:this={importInput}
    class="hidden"
    type="file"
    accept=".floatview,application/octet-stream"
    onchange={importSession}
  />
  {#if backendState.sessionImporting}
    <p class="text-muted" role="status">Importing session…</p>
  {/if}
  {#if backendState.error}
    <p class="text-warning" role="alert">{backendState.error}</p>
  {/if}
  <BoardList />
</section>

<PruneDialog
  open={pruneOpen}
  scope="all"
  error={backendState.error}
  onpreview={(resolution) => backendState.previewSessionPrune('all', resolution)}
  onprune={(resolution) => backendState.startSessionPrune('all', resolution)}
  oncancel={closePrune}
/>
