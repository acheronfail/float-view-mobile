<script lang="ts">
  import { onMount } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import { isLighting, lightingBox } from './lighting';
  import ToggleIndicator from '$lib/ui/ToggleIndicator.svelte';
  import {
    liveWidgetFields,
    type ControllerCurrentLimits,
    type LiveWidgetField,
    type LiveSample,
    type UnitPreference,
  } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import { liveValue, liveValueColor, liveValueText, liveValueStatus } from './live-values';

  let {
    slot,
    selected,
    sample,
    units,
    estimatedLevel,
    currentLimits,
    busy,
    error,
    onselect,
    onclose,
  }: {
    slot: number;
    selected: LiveWidgetField;
    sample: LiveSample | null | undefined;
    units: UnitPreference;
    estimatedLevel?: number | null;
    currentLimits?: ControllerCurrentLimits | null;
    busy: boolean;
    error: string | null;
    onselect: (field: LiveWidgetField, keyboard: boolean) => void;
    onclose: (keyboard: boolean) => void;
  } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="live-value-picker-title"
  aria-describedby="live-value-picker-description"
  aria-busy={busy}
  class="fixed inset-0 m-0 h-dvh max-h-none w-screen max-w-none bg-canvas p-0 text-foreground backdrop:bg-canvas"
  oncancel={(event) => {
    event.preventDefault();
    event.stopPropagation();
    if (!busy) onclose(true);
  }}
>
  <div class="app-safe-area app-safe-area-without-tabs flex h-full min-h-0 flex-col">
    <header class="flex shrink-0 items-center justify-between gap-3 border-b border-outline pb-3">
      <div>
        <h2 id="live-value-picker-title" class="text-xl font-bold">Choose a value</h2>
        <p id="live-value-picker-description" class="mt-1 text-sm text-muted">
          Box {slot + 1} · Tap a reading or control to use it.
        </p>
      </div>
      <button
        type="button"
        class="min-h-11 touch-manipulation rounded-lg border border-outline px-4"
        disabled={busy}
        onpointerdown={(event) => activateFromPointer(event, () => onclose(false))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => onclose(true))}>Cancel</button
      >
    </header>
    {#if error}<p role="alert" class="mt-3 text-sm text-danger">{error}</p>{/if}
    {#if busy}<p role="status" class="mt-3 text-sm text-muted">Saving…</p>{/if}
    <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain py-4">
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
        {#each liveWidgetFields as field (field)}
          {@const reading = liveValue(field, sample, units, estimatedLevel, currentLimits)}
          {@const lighting = isLighting(field) ? lightingBox(backendState.snapshot?.activeConnection, field) : null}
          <button
            type="button"
            aria-label={reading.name}
            aria-pressed={selected === field}
            disabled={busy}
            class="flex min-h-28 min-w-0 touch-pan-y flex-col justify-between gap-2 rounded-xl border p-3 text-left focus-visible:outline-2 focus-visible:outline-accent disabled:opacity-60"
            class:border-accent={selected === field}
            class:bg-accent-soft={selected === field}
            class:border-outline={selected !== field}
            class:bg-surface={selected !== field}
            onpointerdown={(event) => activateFromPointer(event, () => onselect(field, false))}
            onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => onselect(field, true))}
          >
            <span class="text-sm font-medium">{reading.name}</span>
            {#if field === 'empty'}
              <span class="text-sm text-muted">Hold to set</span>
            {:else if lighting}
              {#if lighting.checked != null}<ToggleIndicator checked={lighting.checked} />{/if}
              <span class="text-sm text-muted">{lighting.status}</span>
            {:else}
              <span
                class={`font-mono text-xl font-semibold break-all tabular-nums ${liveValueColor(field, reading.value, sample, units, currentLimits)}`}
              >
                <span class="sr-only">{liveValueStatus(field, sample)} </span>{liveValueText(
                  field,
                  reading.value,
                  reading.digits
                )}<span class="ml-1 text-xs font-normal">{reading.unit}</span>
              </span>
            {/if}
            <span class="text-xs text-muted"
              >{selected === field
                ? '✓ Selected'
                : field === 'empty' || lighting
                  ? '\u00a0'
                  : reading.value == null
                    ? 'Unavailable'
                    : '\u00a0'}</span
            >
          </button>
        {/each}
      </div>
    </div>
  </div>
</dialog>
