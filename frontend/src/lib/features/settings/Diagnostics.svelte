<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import type { DiagnosticList, DiagnosticPreview } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  let opened = $state(false);
  let busy = $state(false);
  let list = $state<DiagnosticList | null>(null);
  let preview = $state<DiagnosticPreview | null>(null);
  let message = $state('');
  let error = $state('');
  let exported = $state(false);
  const button =
    'touch-manipulation min-h-11 rounded border border-outline bg-elevated px-3 py-2 text-foreground disabled:opacity-50';
  async function run(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = '';
    message = '';
    try {
      await action();
    } catch {
      error =
        'Diagnostics could not be completed. The report may have expired or storage may be unavailable. Refresh and try again.';
    } finally {
      busy = false;
    }
  }
  async function refresh() {
    list = await backendState.diagnosticRequest('diagnostics.list', {});
  }
  async function prepare(incidentId: string | null) {
    const replaceReportId = preview?.reportId;
    preview = null;
    preview = await backendState.diagnosticRequest('diagnostics.prepare', {
      incidentId,
      ...(replaceReportId ? { replaceReportId } : {}),
    });
    exported = false;
  }
</script>

<section id="diagnostics" aria-label="Diagnostics" class="space-y-3">
  <SectionTitle title="Diagnostics" />
  <p class="text-sm text-muted">
    Save diagnostic logs to help investigate a reproducible problem. Nothing is uploaded automatically.
  </p>
  <button
    type="button"
    class={button}
    disabled={busy}
    onpointerdown={(e) =>
      activateFromPointer(
        e,
        () =>
          run(async () => {
            opened = true;
            await refresh();
          }),
        { scrollable: true }
      )}
    onclick={(e) =>
      activateFromKeyboardOrAssistiveClick(e, () =>
        run(async () => {
          opened = true;
          await refresh();
        })
      )}>Export diagnostics</button
  >
  {#if opened}
    <p class="text-sm text-muted">
      Includes app/build details, connection and protocol events, failure codes and timing counts. Excludes routes,
      device identifiers, raw packets, configurations and telemetry values. Keeps up to three incidents for seven days.
    </p>
    {#if list?.persistence === 'memoryOnly'}<p class="text-warning">
        Diagnostic storage is unavailable. Recent context is in memory only; file export may be unavailable.
      </p>{/if}
    {#if list && list.droppedRecords > 0}<p class="text-warning">
        {list.droppedRecords} diagnostic records could not be saved.
      </p>{/if}
    <div class="flex flex-wrap gap-2">
      <button
        type="button"
        class={button}
        disabled={busy}
        onpointerdown={(e) =>
          activateFromPointer(
            e,
            () =>
              run(async () => {
                const result = await backendState.diagnosticRequest('diagnostics.capture', {});
                await refresh();
                await prepare(result.incidentId);
              }),
            { scrollable: true }
          )}
        onclick={(e) =>
          activateFromKeyboardOrAssistiveClick(e, () =>
            run(async () => {
              const result = await backendState.diagnosticRequest('diagnostics.capture', {});
              await refresh();
              await prepare(result.incidentId);
            })
          )}>Mark a problem</button
      >
      <button
        type="button"
        class={button}
        disabled={busy}
        onpointerdown={(e) => activateFromPointer(e, () => run(() => prepare(null)), { scrollable: true })}
        onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => run(() => prepare(null)))}
        >Preview recent context</button
      >
      <button
        type="button"
        class={button}
        disabled={busy}
        onpointerdown={(e) => activateFromPointer(e, () => run(refresh), { scrollable: true })}
        onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => run(refresh))}>Refresh incidents</button
      >
    </div>
    {#if list?.incidents.length === 0}<p class="text-sm text-muted">
        No retained incidents. You can mark a problem or preview recent context.
      </p>{/if}
    {#each list?.incidents ?? [] as incident (incident.incidentId)}
      <button
        type="button"
        class={`${button} block w-full text-left`}
        disabled={busy}
        onpointerdown={(e) =>
          activateFromPointer(e, () => run(() => prepare(incident.incidentId)), { scrollable: true })}
        onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => run(() => prepare(incident.incidentId)))}
        >{incident.reason.replaceAll('_', ' ')} · {Math.floor(incident.ageMs / 60_000)} min ago{incident.complete
          ? ''
          : ' · partial capture'}</button
      >
    {/each}
    {#if preview}
      <div class="space-y-3 rounded border border-outline p-3">
        <p class="text-sm text-warning">
          GitHub issues and attachments are public. Review this summary before sharing.
        </p>
        {#if !preview.complete || preview.clipped}<p class="text-sm text-warning">
            This capture is partial or clipped. Preview it again after ten seconds for available follow-up events.
          </p>{/if}
        <pre
          class="text-xs break-words whitespace-pre-wrap"
          aria-label="Diagnostic report preview">{preview.summary}</pre>
        <p class="text-xs text-muted">
          This preview expires after ten minutes. It will not silently include later events.
        </p>
        <div class="flex flex-wrap gap-2">
          <button
            type="button"
            class={button}
            disabled={busy}
            onpointerdown={(e) =>
              activateFromPointer(
                e,
                () =>
                  run(async () => {
                    const result = await backendState.diagnosticRequest('diagnostics.export', {
                      reportId: preview!.reportId,
                    });
                    exported = result.delivery === 'presented';
                    message = exported
                      ? 'Save/share opened. Choose where to save the ZIP.'
                      : 'The diagnostic file is staged.';
                  }),
                { scrollable: true }
              )}
            onclick={(e) =>
              activateFromKeyboardOrAssistiveClick(e, () =>
                run(async () => {
                  const result = await backendState.diagnosticRequest('diagnostics.export', {
                    reportId: preview!.reportId,
                  });
                  exported = result.delivery === 'presented';
                  message = exported
                    ? 'Save/share opened. Choose where to save the ZIP.'
                    : 'The diagnostic file is staged.';
                })
              )}>Save/share ZIP</button
          >
          <button
            type="button"
            class={button}
            disabled={busy}
            onpointerdown={(e) =>
              activateFromPointer(
                e,
                () =>
                  run(async () => {
                    await navigator.clipboard.writeText(preview!.summary);
                    message = 'Summary copied.';
                  }),
                { scrollable: true }
              )}
            onclick={(e) =>
              activateFromKeyboardOrAssistiveClick(e, () =>
                run(async () => {
                  await navigator.clipboard.writeText(preview!.summary);
                  message = 'Summary copied.';
                })
              )}>Copy summary</button
          >
        </div>
      </div>
    {/if}
    {#if exported}
      <div class="space-y-2 text-sm text-muted">
        <p>
          To report a reproducible bug, first check existing <a
            class="touch-manipulation text-accent underline"
            href="https://github.com/acheronfail/float-view-mobile/issues"
            target="_blank"
            rel="noreferrer">GitHub Issues</a
          >.
        </p>
        <p>
          If it has not been reported, manually write an issue with steps to reproduce, expected and actual behavior,
          and any known controller, firmware, motor and battery details. Unknown details are fine. Attach the saved ZIP
          in the issue's attachment area, then review and submit it on GitHub. Avoid private screenshots or route files.
        </p>
      </div>
    {/if}
    <button
      type="button"
      class={button}
      disabled={busy}
      onpointerdown={(e) =>
        activateFromPointer(
          e,
          () =>
            run(async () => {
              await backendState.diagnosticRequest('diagnostics.clear', {});
              preview = null;
              exported = false;
              await refresh();
              message = 'Retained diagnostics cleared. Saved/shared copies remain with their recipients.';
            }),
          { scrollable: true }
        )}
      onclick={(e) =>
        activateFromKeyboardOrAssistiveClick(e, () =>
          run(async () => {
            await backendState.diagnosticRequest('diagnostics.clear', {});
            preview = null;
            exported = false;
            await refresh();
            message = 'Retained diagnostics cleared. Saved/shared copies remain with their recipients.';
          })
        )}>Clear retained diagnostics</button
    >
  {/if}
  {#if message}<p role="status" class="text-sm text-muted">{message}</p>{/if}
  {#if error}<p role="alert" class="text-sm text-warning">{error}</p>{/if}
</section>
