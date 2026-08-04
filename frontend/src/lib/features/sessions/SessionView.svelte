<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import MenuItem from '$lib/ui/MenuItem.svelte';
  import StickyHeader from '$lib/ui/StickyHeader.svelte';
  import { sessionExportLabel } from './session-export';
  import SessionHistory from './SessionHistory.svelte';
  import SessionEvents from './SessionEvents.svelte';
  import SessionLocationMap from './SessionLocationMap.svelte';
  import type { Viewport } from './viewport';

  let { sessionId }: { sessionId: string } = $props();
  let menuButton = $state<HTMLButtonElement>();
  let deleteConfirmationOpen = $state(false);
  let deleting = $state(false);
  let sessionHeaderHeight = $state(0);
  let locationMapHeight = $state(0);
  let selectedElapsedMs = $state(0);
  let chartViewport = $state<Viewport | null>(null);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

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
    menuButton?.focus();
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
  aria-labelledby="session-heading"
  class="space-y-2"
  style={`--section-header-top: calc(max(1.5rem, env(safe-area-inset-top, 0px)) + ${sessionHeaderHeight + locationMapHeight}px)`}
>
  {#if backendState.selectedSession}
    <StickyHeader
      bind:button={menuButton}
      bind:headerHeight={sessionHeaderHeight}
      headingId="session-heading"
      title={backendState.selectedSession.displayName}
      titleClass="mt-2 text-lg"
      backHref="#/sessions"
      backLabel="← Sessions"
      menuLabel="Session actions"
    >
      {#snippet menu(closeMenu)}
        <MenuItem
          label={sessionExportLabel(backendState.platform, 'float-view-archive')}
          disabled={backendState.busy || backendState.selectedSession?.state === 'active'}
          onactivate={() => exportSession(closeMenu, 'float-view-archive')}
        />
        <MenuItem
          label={sessionExportLabel(backendState.platform, 'csv')}
          disabled={backendState.busy || backendState.selectedSession?.state === 'active'}
          onactivate={() => exportSession(closeMenu, 'csv')}
        />
        <MenuItem
          label="Delete session"
          appearance="destructive"
          disabled={backendState.busy || backendState.selectedSession?.state === 'active'}
          onactivate={() => requestDeleteSession(closeMenu)}
        />
      {/snippet}
    </StickyHeader>

    {#if backendState.sessionMetadata}
      {#if backendState.sessionMetadata.location.available}
        <SessionLocationMap
          metadata={backendState.sessionMetadata}
          {selectedElapsedMs}
          viewport={chartViewport}
          stickyTop={sessionHeaderHeight}
          bind:height={locationMapHeight}
        />
      {/if}

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

      <div class="mt-3">
        <SessionHistory
          metadata={backendState.sessionMetadata}
          onselectionchange={(elapsedMs) => (selectedElapsedMs = elapsedMs)}
          onviewportchange={(viewport) => (chartViewport = viewport)}
        />
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
