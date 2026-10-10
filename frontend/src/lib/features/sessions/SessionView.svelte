<script lang="ts">
  import type { SessionDisplayOptions, SessionRideMarker } from '$lib/backend/contract';
  import { SessionDataHttpError } from '$lib/backend/session-data-client';
  import { ApiRequestError } from '$lib/backend/client';
  import SessionMarkers from './SessionMarkers.svelte';
  import { getContext, onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import PruneDialog from './PruneDialog.svelte';
  import { hasCoarserPruneResolution } from './prune';
  import { sessionExportLabel } from './session-export';
  import SessionHistory from './SessionHistory.svelte';
  import SessionEvents from './SessionEvents.svelte';
  import SessionLocationMap from './SessionLocationMap.svelte';
  import { sessionStartDateLabel } from './session-label';
  import type { Viewport } from './viewport';
  import type { SessionLocationPoint } from '$lib/backend/contract';
  import { mapIntervals, type MapBounds } from './map-filter';
  let filterByMap = $state(false);
  let mapBounds = $state<MapBounds | null>(null);
  let track = $state<SessionLocationPoint[][] | null>(null);
  let trackSnapshot = $state('');
  let refreshAttempted = false;
  const snapshotKey = $derived(JSON.stringify(backendState.sessionMetadata?.snapshot));
  const spatialSelection = $derived.by(() => {
    if (!filterByMap || !mapBounds || !track || trackSnapshot !== snapshotKey) return { intervals: null, error: null };
    try {
      return { intervals: mapIntervals(track, mapBounds), error: null };
    } catch (reason) {
      return { intervals: [], error: reason instanceof Error ? reason.message : String(reason) };
    }
  });
  function setMapFilter(enabled: boolean) {
    filterByMap = enabled;
    refreshAttempted = false;
  }
  function refreshSpatialSnapshot(reason: unknown) {
    const conflict =
      (reason instanceof ApiRequestError && reason.code === 'conflict') ||
      (reason instanceof SessionDataHttpError && reason.status === 409);
    if (!conflict || refreshAttempted) return;
    refreshAttempted = true;
    void backendState.openSession(sessionId);
  }

  let { sessionId, boardSavedDeviceId }: { sessionId: string; boardSavedDeviceId?: string | null } = $props();
  let deleteConfirmationOpen = $state(false);
  let pruneOpen = $state(false);
  let deleting = $state(false);
  let savingDisplayOptions = $state(false);
  async function toggleDisplayOption(key: keyof SessionDisplayOptions) {
    const settings = backendState.settings;
    if (!settings || savingDisplayOptions) return;
    savingDisplayOptions = true;
    try {
      await backendState.updateSessionSettings({
        sessionDisplayOptions: { ...settings.sessionDisplayOptions, [key]: !settings.sessionDisplayOptions[key] },
      });
    } finally {
      savingDisplayOptions = false;
    }
  }
  let locationMapHeight = $state(0);
  let selectedElapsedMs = $state(0);
  let markers = $state<SessionRideMarker[]>([]);
  let selectedMarker = $state<SessionRideMarker | null>(null);
  let selectedPoint = $state<{ elapsedMs: number } | null>(null);
  let markerError = $state('');
  let markerRetry = '';
  function selectMarker(marker: SessionRideMarker) {
    selectedMarker = marker;
    selectedElapsedMs = marker.elapsedMs;
    selectedPoint = { elapsedMs: marker.elapsedMs };
  }
  function selectFault(elapsedMs: number) {
    selectedMarker = null;
    selectedElapsedMs = elapsedMs;
    selectedPoint = { elapsedMs };
  }
  $effect(() => {
    const metadata = backendState.sessionMetadata;
    if (!metadata || metadata.summary.sessionId !== sessionId) return;
    const controller = backendState.createSessionReviewController(metadata);
    let active = true;
    markers = [];
    selectedMarker = null;
    selectedPoint = null;
    markerError = '';
    void (async () => {
      const loaded: SessionRideMarker[] = [];
      let cursor: string | null = null;
      for (let page = 0; page < 4 && metadata.markerCount; page++) {
        const result = await controller.loadMarkers({
          kind: 'markers',
          range: { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs },
          limit: 256,
          cursor,
        });
        if (!active) return;
        loaded.push(...result.markers);
        cursor = result.nextCursor;
        if (!cursor) break;
      }
      if (cursor || loaded.length !== metadata.markerCount || loaded.length > 1000)
        throw new Error('Incomplete marker history');
      if (active) markers = loaded;
    })().catch((error) => {
      if (!active) return;
      if (
        error instanceof ApiRequestError &&
        ['conflict', 'invalid_request'].includes(error.code) &&
        markerRetry !== metadata.summary.sessionId
      ) {
        markerRetry = metadata.summary.sessionId;
        void backendState.openSession(metadata.summary.sessionId);
      } else markerError = 'Markers could not be loaded.';
    });
    return () => {
      active = false;
      controller.dispose();
    };
  });
  let chartViewport = $state<Viewport | null>(null);
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/sessions/[sessionId]';
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const owningBoardSavedDeviceId = $derived(
    boardSavedDeviceId ??
      backendState.savedDevices.find(
        (device) => device.deviceHistoryId === backendState.selectedSession?.deviceHistoryId
      )?.savedDeviceId ??
      null
  );
  const returnHref = $derived(
    owningBoardSavedDeviceId ? `#/sessions/boards/${encodeURIComponent(owningBoardSavedDeviceId)}` : '#/sessions'
  );

  $effect(() => {
    const session = backendState.selectedSession;
    const canExport = !backendState.busy && session?.state !== 'active';
    appHeader.set(headerOwner, {
      title: sessionStartDateLabel(session?.startedAt) ?? 'Session',
      backHref: returnHref,
      backLabel: owningBoardSavedDeviceId ? 'Board sessions' : 'Sessions',
      ...(session
        ? {
            menuLabel: 'Session actions',
            menu: [
              {
                label: 'Both footpads off at speed',
                checked: backendState.settings?.sessionDisplayOptions.bothFootpadsOff ?? false,
                disabled: savingDisplayOptions,
                onactivate: () => toggleDisplayOption('bothFootpadsOff'),
              },
              {
                label: 'One footpad off at speed',
                checked: backendState.settings?.sessionDisplayOptions.oneFootpadOff ?? false,
                disabled: savingDisplayOptions,
                onactivate: () => toggleDisplayOption('oneFootpadOff'),
              },
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
                label: 'Prune',
                appearance: 'warning' as const,
                disabled:
                  !canExport ||
                  !hasCoarserPruneResolution(backendState.sessionMetadata?.prune.targetResolutionSeconds ?? null),
                onactivate: (closeMenu: () => void) => requestPruneSession(closeMenu),
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
    if (!backendState.sessionMetadata?.location.available && !markers.some((m) => m.location)) locationMapHeight = 0;
  });

  $effect(() => {
    if (backendState.sessionMetadata?.summary.sessionId !== sessionId) {
      chartViewport = null;
      filterByMap = false;
      mapBounds = null;
      track = null;
      trackSnapshot = '';
      refreshAttempted = false;
    }
  });

  onMount(() => {
    let active = true;
    void backendState.openSession(sessionId).then((opened) => {
      if (active && !opened) void goto(returnHref, { replaceState: true });
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

  function requestPruneSession(closeMenu: () => void) {
    closeMenu();
    if (backendState.selectedSession) pruneOpen = true;
  }

  async function closePruneSession() {
    pruneOpen = false;
    await tick();
    appHeader.focusMenu();
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
    if (deleted) await goto(returnHref, { replaceState: true });
  }
</script>

<section
  aria-labelledby="app-title"
  class="space-y-2 py-2"
  style={`--section-header-top: calc(var(--app-header-bottom) + ${locationMapHeight}px)`}
>
  {#if backendState.selectedSession}
    {#if backendState.sessionMetadata}
      <SessionMarkers {markers} selected={selectedMarker?.sequence ?? null} onselect={selectMarker} />
      {#if markerError}<p role="alert" class="text-warning">{markerError}</p>{/if}
      {#if backendState.sessionMetadata.location.available || markers.some((m) => m.location)}
        {#key `${backendState.sessionMetadata.summary.sessionId}:${snapshotKey}`}
          <SessionLocationMap
            metadata={backendState.sessionMetadata}
            {selectedElapsedMs}
            {markers}
            {selectedMarker}
            onmarkerselect={selectMarker}
            onfaultselect={selectFault}
            viewport={chartViewport}
            {filterByMap}
            initialBounds={mapBounds}
            onfilterchange={setMapFilter}
            onbounds={(bounds) => {
              mapBounds = bounds;
            }}
            ontrack={(segments) => {
              track = segments;
              trackSnapshot = snapshotKey;
            }}
            ontrackerror={refreshSpatialSnapshot}
            bind:height={locationMapHeight}
          />
        {/key}
      {:else}
        <div class="rounded-lg border border-outline bg-elevated p-3">No location data was saved for this ride.</div>
      {/if}

      <div class="mt-3">
        {#key `${backendState.sessionMetadata.summary.sessionId}:${snapshotKey}`}
          <SessionHistory
            metadata={backendState.sessionMetadata}
            {markers}
            {selectedPoint}
            {filterByMap}
            intervals={spatialSelection.intervals}
            filterError={spatialSelection.error}
            ondataerror={refreshSpatialSnapshot}
            initialViewport={chartViewport}
            onmarkerselect={selectMarker}
            onselectionchange={(elapsedMs) => {
              selectedElapsedMs = elapsedMs;
              if (selectedMarker?.elapsedMs !== elapsedMs) selectedMarker = null;
            }}
            onviewportchange={(viewport) => (chartViewport = viewport)}
          />
        {/key}
      </div>

      {#if filterByMap}<p class="mt-3 text-xs text-muted">
          Session totals and events are not filtered by the map.
        </p>{/if}
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

    {#if backendState.sessionMetadata}<SessionEvents
        metadata={backendState.sessionMetadata}
        viewport={chartViewport}
        {filterByMap}
      />{/if}
  {:else}
    <p class="mt-3 text-muted">Loading session…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>

<PruneDialog
  open={pruneOpen}
  scope="one"
  existingResolutionSeconds={backendState.sessionMetadata?.prune.targetResolutionSeconds}
  error={backendState.error}
  onpreview={(resolution) => backendState.previewSessionPrune('one', resolution, sessionId)}
  onprune={(resolution) => backendState.startSessionPrune('one', resolution, sessionId)}
  oncancel={closePruneSession}
/>

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
