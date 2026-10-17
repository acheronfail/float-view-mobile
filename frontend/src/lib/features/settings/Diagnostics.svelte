<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';

  let busy = $state(false);
  let message = $state('');
  let error = $state('');
  let report: { id: string; expires: number } | null = null;

  async function exportDiagnostics() {
    if (busy) return;
    busy = true;
    error = '';
    message = '';
    try {
      const list = await backendState.diagnosticRequest('diagnostics.list', {});
      const previous = report;
      report = null;
      const prepared = await backendState.diagnosticRequest('diagnostics.prepare', {
        incidentId: list.incidents[0]?.incidentId ?? null,
        ...(previous && Date.now() < previous.expires ? { replaceReportId: previous.id } : {}),
      });
      report = { id: prepared.reportId, expires: Date.now() + prepared.expiresInMs };
      const result = await backendState.diagnosticRequest('diagnostics.export', { reportId: prepared.reportId });
      message =
        result.delivery === 'presented' ? 'Choose where to save or share the ZIP.' : 'The diagnostic ZIP is ready.';
    } catch {
      error = 'Could not export diagnostics. Please try again.';
    } finally {
      busy = false;
    }
  }
</script>

<section id="diagnostics" aria-label="Diagnostics" class="space-y-3">
  <SectionTitle title="Diagnostics" />
  <p class="text-sm text-muted">
    Export logs to help investigate a bug. Logs exclude routes and device identifiers. Nothing is uploaded
    automatically.
  </p>
  <button
    type="button"
    class="min-h-11 touch-manipulation rounded border border-outline bg-elevated px-3 py-2 text-foreground disabled:opacity-50"
    disabled={busy}
    onpointerdown={(e) => activateFromPointer(e, exportDiagnostics, { scrollable: true })}
    onclick={(e) => activateFromKeyboardOrAssistiveClick(e, exportDiagnostics)}
    >{busy ? 'Exporting…' : 'Export diagnostics'}</button
  >
  <p class="text-sm text-muted">
    To report a bug, check existing <a
      class="touch-manipulation text-accent underline"
      href="https://github.com/acheronfail/float-view-mobile/issues"
      target="_blank"
      rel="noreferrer">GitHub Issues</a
    >, then manually create an issue with steps to reproduce and attach the ZIP. Issues and attachments are public.
  </p>
  {#if message}<p role="status" class="text-sm text-muted">{message}</p>{/if}
  {#if error}<p role="alert" class="text-sm text-warning">{error}</p>{/if}
</section>
