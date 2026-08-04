<script lang="ts">
  import { getContext, onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { sessionExportLabel } from './session-export';
  import SessionHistory from './SessionHistory.svelte';
  import SessionEvents from './SessionEvents.svelte';
  import SessionLocationMap from './SessionLocationMap.svelte';
  import type { Viewport } from './viewport';

  let { sessionId }: { sessionId: string } = $props();
  let deleteConfirmationOpen = $state(false);
  let deleting = $state(false);
  let locationMapHeight = $state(0);
  let selectedElapsedMs = $state(0);
  let chartViewport = $state<Viewport | null>(null);
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/sessions/[sessionId]';
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

  $effect(() => {
    const session = backendState.selectedSession;
    const canExport = !backendState.busy && session?.state !== 'active';
    appHeader.set(headerOwner, {
      title: session?.displayName ?? 'Session',
      backHref: '#/sessions',
      backLabel: '← Sessions',
      ...(session
        ? {
            menuLabel: 'Session actions',
            menu: [
              {
                label: sessionExportLabel(backendState.platform, 'float-view-archive'),
                disabled: !canExport,
                onactivate: (closeMenu: () => void) => exportSession(closeMenu, 'float-view-archive'),
              },
              {
                label: sessionExportLabel(backendState.platform, 'csv'),
                disabled: !canExport,
                onactivate: (closeMenu: () => void) => exportSession(closeMenu, 'csv'),
              },
              {
                label: 'Delete session',
                appearance: 'destructive' as const,
                disabled: !canExport,
                onactivate: (closeMenu: () => void) => requestDeleteSession(closeMenu),
              },
            ],
          }
        : {}),
    });
  });
  onDestroy(() => appHeader.clear(headerOwner));

  $effect(() => {
    if (!backendState.sessionMetadata?.location.available) locationMapHeight = 0;
  });

  $effect(() => {
    if (backendState.sessionMetadata?.summary.sessionId !== sessionId) chartViewport = null;
  });

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

  function requestDeleteSession(closeMenu: () => void) {
    closeMenu();
    if (backendState.selectedSession) deleteConfirmationOpen = true;
  }

  async function cancelDeleteSession() {
    deleteConfirmationOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function exportSession(closeMenu: () => void, format: 'float-view-archive' | 'csv') {
    closeMenu();
    await backendState.exportSession(format);
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

<section
  aria-labelledby="app-title"
  class="space-y-2"
  style={`--section-header-top: calc(var(--app-header-bottom) + ${locationMapHeight}px)`}
>
  {#if backendState.selectedSession}
    {#if backendState.sessionMetadata}
      {#if backendState.sessionMetadata.location.available}
        <SessionLocationMap
          metadata={backendState.sessionMetadata}
          {selectedElapsedMs}
          viewport={chartViewport}
          bind:height={locationMapHeight}
        />
        {:else}
        <div class="rounded-lg border border-outline bg-elevated p-3">
          No location data was saved for this ride.
        </div>
      {/if}

      <div class="mt-3">
        <SessionHistory
          metadata={backendState.sessionMetadata}
          onselectionchange={(elapsedMs) => (selectedElapsedMs = elapsedMs)}
          onviewportchange={(viewport) => (chartViewport = viewport)}
        />
      </div>

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
    {/if}

    {#if backendState.sessionMetadata}<SessionEvents metadata={backendState.sessionMetadata} />{/if}
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
  appearance="destructive"
  busy={deleting}
  onconfirm={deleteSession}
  oncancel={cancelDeleteSession}
/>
