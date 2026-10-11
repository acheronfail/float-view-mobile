<script lang="ts">
  import { tick } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import {
    defaultLiveWidgetSlots,
    type LiveSample,
    type LiveWidgetField,
    type LiveWidgetSlots,
  } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import LiveNumber from './LiveNumber.svelte';
  import LiveValuePicker from './LiveValuePicker.svelte';
  import { liveValueCatalog } from './live-values';
  import { holdToConfigure } from './hold-to-configure';
  import { isLighting, lightingBox } from './lighting';
  import ToggleIndicator from '$lib/ui/ToggleIndicator.svelte';

  let { sample, slot }: { sample: LiveSample | null | undefined; slot: number } = $props();
  const slots = $derived(backendState.settings?.liveWidgetSlots ?? defaultLiveWidgetSlots);
  const field = $derived(slots[slot]);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const connection = $derived(backendState.snapshot?.activeConnection);
  const lighting = $derived(isLighting(field) ? lightingBox(connection, field) : null);
  const currentLimits = $derived(backendState.snapshot?.activeConnection?.currentLimits);
  const estimatedLevel = $derived(backendState.snapshot?.activeConnection?.batteryEstimate.level);
  let selectedSlot = $state<number | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let trigger: HTMLButtonElement | undefined;
  let button: HTMLButtonElement;
  let toggling = $state(false);
  async function toggleLighting() {
    if (!isLighting(field) || !lighting || lighting.disabled || toggling || backendState.busy) return;
    toggling = true;
    await backendState.setRefloatControl(field, !lighting.checked);
    toggling = false;
  }
  function keyboardActivate() {
    if (lighting) void toggleLighting();
    else open(button);
  }

  function open(button: HTMLButtonElement) {
    trigger = button;
    error = null;
    selectedSlot = slot;
  }
  async function close(keyboard: boolean) {
    if (busy) return;
    selectedSlot = null;
    await tick();
    if (keyboard) trigger?.focus();
    else trigger?.blur();
  }
  async function select(field: LiveWidgetField, keyboard: boolean) {
    if (busy || selectedSlot == null) return;
    if (slots[selectedSlot] === field) {
      await close(keyboard);
      return;
    }
    const updated: LiveWidgetSlots = [...slots];
    updated[selectedSlot] = field;
    busy = true;
    error = null;
    const saved = await backendState.updateSessionSettings({ liveWidgetSlots: updated });
    busy = false;
    if (saved) await close(keyboard);
    else error = backendState.error ?? 'Could not save this reading. Try again.';
  }
</script>

<div class="min-w-0">
  <button
    bind:this={button}
    use:holdToConfigure={{
      key: `${slot}:${field}:${connection?.deviceId ?? ''}`,
      onhold: () => open(button),
      ontap: () => void toggleLighting(),
    }}
    type="button"
    role={lighting?.checked != null ? 'switch' : undefined}
    aria-checked={lighting?.checked ?? undefined}
    aria-busy={lighting ? lighting.pending || toggling : undefined}
    aria-disabled={lighting ? lighting.disabled || toggling || backendState.busy : undefined}
    aria-label={lighting
      ? liveValueCatalog[field].name
      : `Choose value for box ${slot + 1}: ${liveValueCatalog[field].name}`}
    aria-describedby={`live-number-${slot} live-box-help-${slot}`}
    aria-keyshortcuts="Shift+Enter"
    class="block min-h-11 w-full min-w-0 touch-pan-y rounded-lg border border-outline bg-surface px-1 py-1.5 text-center select-none focus-visible:outline-2 focus-visible:outline-accent"
    style="-webkit-touch-callout: none"
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, keyboardActivate)}
    onkeydown={(event) => {
      if (
        (event.key === 'Enter' && event.shiftKey) ||
        event.key === 'ContextMenu' ||
        (event.key === 'F10' && event.shiftKey)
      ) {
        event.preventDefault();
        if (!event.repeat) open(button);
      }
    }}
  >
    <div id={`live-number-${slot}`} role="group" aria-label={liveValueCatalog[field].name}>
      {#if field === 'empty'}
        <span class="flex min-h-10 items-center justify-center text-xs text-muted">Hold to set</span>
      {:else if lighting}
        <div
          class="flex min-h-10 flex-col items-center justify-center gap-1"
          class:opacity-50={lighting.disabled && !lighting.pending}
        >
          <span class="text-[10px] text-muted uppercase">{liveValueCatalog[field].label}</span>
          {#if lighting.checked != null}
            <ToggleIndicator checked={lighting.checked} />
            {#if lighting.pending || toggling}<span class="text-[10px] text-muted">Pending…</span>{/if}
          {:else}<span class="text-xs text-muted">{lighting.status}</span>{/if}
        </div>
      {:else}
        <LiveNumber {field} {sample} {units} {estimatedLevel} {currentLimits} />
      {/if}
    </div>
  </button>
  <span id={`live-box-help-${slot}`} class="sr-only"
    >Hold to configure. Shift+Enter opens configuration.{lighting ? ' Tap or press Enter to toggle.' : ''}</span
  >
  {#if lighting}
    <button
      type="button"
      class="sr-only focus:not-sr-only focus:bg-surface"
      aria-label={`Configure box ${slot + 1}: ${liveValueCatalog[field].name}`}
      onpointerdown={(event) => activateFromPointer(event, () => open(button))}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => open(button))}>Configure</button
    >
  {/if}
</div>
{#if selectedSlot != null}
  <LiveValuePicker
    slot={selectedSlot}
    selected={slots[selectedSlot]}
    {sample}
    {units}
    {estimatedLevel}
    {currentLimits}
    {busy}
    {error}
    onselect={select}
    onclose={close}
  />
{/if}
