<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';

  let busy = $state(false);
  let error = $state('');
  let report: { id: string; expires: number } | null = null;

  async function exportDiagnostics() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const list = await backendState.diagnosticRequest('diagnostics.list', {});
      const previous = report;
      report = null;
      const prepared = await backendState.diagnosticRequest('diagnostics.prepare', {
        incidentId: list.incidents[0]?.incidentId ?? null,
        ...(previous && Date.now() < previous.expires ? { replaceReportId: previous.id } : {}),
      });
      report = { id: prepared.reportId, expires: Date.now() + prepared.expiresInMs };
      await backendState.diagnosticRequest('diagnostics.export', { reportId: prepared.reportId });
    } catch {
      error = 'Could not export diagnostics. Please try again.';
    } finally {
      busy = false;
    }
  }
</script>

<section id="diagnostics" aria-label="Diagnostics" class="space-y-3">
  <SectionTitle title="Help with a problem" />
  <p class="text-sm text-muted">
    Save a troubleshooting file to help us understand what went wrong. It doesn’t include device identifiers. Nothing is
    sent automatically.
  </p>
  <button
    type="button"
    class="min-h-11 touch-manipulation rounded border border-outline bg-elevated px-3 py-2 text-foreground disabled:opacity-50"
    disabled={busy}
    onpointerdown={(e) => activateFromPointer(e, exportDiagnostics, { scrollable: true })}
    onclick={(e) => activateFromKeyboardOrAssistiveClick(e, exportDiagnostics)}
    >{busy ? 'Saving…' : 'Save troubleshooting file'}</button
  >
  <p class="text-sm text-muted">
    Check <a
      class="touch-manipulation text-accent underline"
      href="https://github.com/acheronfail/float-view-mobile/issues"
      target="_blank"
      rel="noreferrer">GitHub Issues</a
    > to see if someone has already reported the problem. If not, add a new report, explain what happened and attach the
    saved file. Anyone can see reports and attached files.
  </p>
  {#if error}<p role="alert" class="text-sm text-warning">{error}</p>{/if}
</section>
