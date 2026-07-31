<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import TelemetryChart from './TelemetryChart.svelte';
  import { extensionHistoricalFields, historicalTelemetryGroups, sessionEventText } from './telemetry';

  let { sessionId }: { sessionId: string } = $props();
  let deleteConfirmationOpen = $state(false);
  let deleting = $state(false);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const extensionFields = $derived(extensionHistoricalFields(backendState.sessionSeries));

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

  async function inspectMidpoint() {
    const session = backendState.selectedSession;
    if (!session) return;
    await backendState.loadDiagnosticWindow(session.totalElapsedMs / 2);
    document.getElementById('diagnostic-window')?.scrollIntoView({ behavior: 'smooth' });
  }

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

<section aria-labelledby="session-heading" class="rounded-2xl border border-outline bg-surface p-5">
  <a class="text-accent" href="#/sessions">← Sessions</a>
  {#if backendState.selectedSession}
    <h2 id="session-heading" class="mt-3 text-xl font-medium">Review</h2>
    <p class="mt-2 text-lg">{backendState.selectedSession.displayName}</p>
    <div class="mt-4 grid grid-cols-2 gap-3">
      <div class="rounded-lg bg-elevated p-3">
        <p class="text-sm text-muted">Samples</p>
        <p class="text-xl">{backendState.selectedSession.sampleCount}</p>
      </div>
      <div class="rounded-lg bg-elevated p-3">
        <p class="text-sm text-muted">Distance</p>
        <p class="text-xl">
          {formatDistanceM(backendState.selectedSession.distanceM, units)}
        </p>
      </div>
    </div>

    <div class="mt-5 rounded-lg border border-outline p-4">
      <h3 class="font-medium">Detailed telemetry</h3>
      {#if backendState.sessionSeries.length}
        <p class="mt-2 text-xs text-muted">{backendState.sessionSeries.length} bounded points</p>
        <div class="mt-3 space-y-4">
          {#each historicalTelemetryGroups as group (group.title)}
            <TelemetryChart title={group.title} fields={group.fields} points={backendState.sessionSeries} />
          {/each}
          {#if extensionFields.length}
            <TelemetryChart
              title="Negotiated extensions"
              fields={extensionFields}
              points={backendState.sessionSeries}
            />
          {/if}
        </div>
      {:else}
        <p class="mt-2 text-sm text-muted">No samples were recorded for this session.</p>
      {/if}
      <button
        type="button"
        class="mt-3 touch-manipulation rounded-md bg-elevated px-3 py-2 disabled:opacity-50"
        disabled={backendState.busy || backendState.selectedSession.totalElapsedMs === 0}
        onpointerdown={(event) => activateFromPointer(event, inspectMidpoint)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, inspectMidpoint)}
        >Inspect midpoint ±15 seconds</button
      >
      {#if backendState.diagnosticSeries.length}
        <div id="diagnostic-window" class="mt-4 rounded bg-canvas p-3">
          <p class="text-sm font-medium">Diagnostic window</p>
          <p class="mt-1 text-xs text-muted">
            {backendState.diagnosticSeries.length} lossless samples
          </p>
        </div>
      {/if}
    </div>

    <div class="mt-5 rounded-lg border border-outline p-4">
      <h3 class="font-medium">State transitions</h3>
      {#if backendState.sessionEventsTruncated}
        <p role="status" class="mt-2 text-sm text-warning">
          Earlier transitions were truncated; the newest 2,000 are shown.
        </p>
      {/if}
      {#if backendState.sessionEvents.length}
        <ol class="mt-3 space-y-2">
          {#each backendState.sessionEvents as event, index (`${event.elapsedMs}-${event.kind}-${index}`)}
            <li class="flex gap-3 rounded bg-elevated px-3 py-2 text-sm">
              <time class="shrink-0 font-mono text-xs text-muted">
                {(event.elapsedMs / 1_000).toFixed(1)} s
              </time>
              <span>{sessionEventText(event)}</span>
            </li>
          {/each}
        </ol>
      {:else}
        <p class="mt-2 text-sm text-muted">No state transitions were recorded.</p>
      {/if}
    </div>

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
