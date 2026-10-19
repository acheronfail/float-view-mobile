<script lang="ts">
  import type { SessionPrunePreview } from '$lib/backend/contract';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { formatPruneBytes, suggestedPruneResolution, validPruneResolution } from './prune';

  let {
    open,
    scope,
    existingResolutionSeconds = null,
    error = null,
    onpreview,
    onprune,
    oncancel,
  }: {
    open: boolean;
    scope: 'one' | 'all';
    existingResolutionSeconds?: number | null;
    error?: string | null;
    onpreview: (targetResolutionSeconds: number) => Promise<SessionPrunePreview | null>;
    onprune: (targetResolutionSeconds: number) => Promise<boolean>;
    oncancel: () => void | Promise<void>;
  } = $props();

  let targetResolutionSeconds = $state(5);
  let preview = $state<SessionPrunePreview | null>(null);
  let previewResolution = $state<number | null>(null);
  let operationError = $state<string | null>(null);
  let busy = $state(false);
  let wasOpen = false;

  const resolutionValid = $derived(validPruneResolution(targetResolutionSeconds, existingResolutionSeconds));
  const previewCurrent = $derived(preview !== null && previewResolution === targetResolutionSeconds);

  $effect(() => {
    if (open && !wasOpen) {
      targetResolutionSeconds = suggestedPruneResolution(existingResolutionSeconds);
      operationError = null;
      preview = null;
      previewResolution = null;
    }
    wasOpen = open;
  });

  function resolutionChanged() {
    preview = null;
    previewResolution = null;
  }

  async function proceed() {
    if (busy || !resolutionValid) return;
    busy = true;
    operationError = null;
    try {
      if (!previewCurrent) {
        preview = await onpreview(targetResolutionSeconds);
        previewResolution = preview ? targetResolutionSeconds : null;
        if (!preview) operationError = error ?? 'The prune preview could not be loaded. Please try again.';
        return;
      }
      if (await onprune(targetResolutionSeconds)) await oncancel();
      else operationError = error ?? 'Sessions could not be pruned. Please try again.';
    } catch (cause) {
      operationError = cause instanceof Error ? cause.message : 'Sessions could not be pruned. Please try again.';
    } finally {
      busy = false;
    }
  }
</script>

<ConfirmDialog
  {open}
  titleId="prune-sessions-title"
  title={scope === 'all' ? 'Prune all sessions?' : 'Prune session?'}
  description="Pruning lowers time resolution, but does not simply discard arbitrary points. Float View keeps real source samples that best preserve important minimum and maximum values, ride-state transitions, events, gaps, boundaries, and the first appearance of fields. Fine detail between retained samples cannot be recovered."
  confirmLabel={previewCurrent ? (scope === 'all' ? 'Prune all' : 'Prune') : 'Preview'}
  appearance="warning"
  {busy}
  confirmDisabled={!resolutionValid}
  error={operationError}
  onconfirm={proceed}
  {oncancel}
>
  <label for="prune-resolution" class="block text-sm font-medium">Target resolution</label>
  <div class="mt-2 flex items-center gap-2">
    <input
      id="prune-resolution"
      type="number"
      inputmode="numeric"
      min="1"
      max="300"
      step="1"
      bind:value={targetResolutionSeconds}
      onchange={resolutionChanged}
      class="min-w-0 flex-1 rounded-lg border border-outline bg-canvas px-3 py-2"
    />
    <span class="text-sm text-muted">seconds</span>
  </div>
  {#if existingResolutionSeconds}
    <p class="mt-2 text-xs leading-5 text-muted">
      This session is already at {existingResolutionSeconds} seconds. Choose a larger multiple of that resolution.
    </p>
  {:else}
    <p class="mt-2 text-xs leading-5 text-muted">Choose a whole-number resolution from 1 second to 5 minutes.</p>
  {/if}
  {#if !resolutionValid}
    <p class="mt-2 text-sm text-warning" role="alert">Choose a valid target resolution before previewing.</p>
  {/if}
  {#if previewCurrent && preview}
    <div class="mt-4 rounded-lg border border-outline bg-canvas p-3 text-sm">
      {#if preview.skipped}
        <p>No samples would be removed at this resolution.</p>
      {:else}
        <p>
          Estimated samples: {preview.recordedSampleCount.toLocaleString()} → {preview.estimatedStoredSampleCount.toLocaleString()}
        </p>
        <p class="mt-1 text-muted">{formatPruneBytes(preview.estimatedReclaimableBytes)}</p>
      {/if}
    </div>
  {/if}
</ConfirmDialog>
