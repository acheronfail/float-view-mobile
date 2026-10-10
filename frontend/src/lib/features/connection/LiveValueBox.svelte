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

  let { sample, slot }: { sample: LiveSample | null | undefined; slot: number } = $props();
  const slots = $derived(backendState.settings?.liveWidgetSlots ?? defaultLiveWidgetSlots);
  const field = $derived(slots[slot]);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const currentLimits = $derived(backendState.snapshot?.activeConnection?.currentLimits);
  const estimatedLevel = $derived(backendState.snapshot?.activeConnection?.batteryEstimate.level);
  let selectedSlot = $state<number | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let trigger: HTMLButtonElement | undefined;

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

<button
  type="button"
  aria-label={`Choose value for box ${slot + 1}: ${liveValueCatalog[field].name}`}
  aria-describedby={`live-number-${slot}`}
  class="block min-h-11 w-full min-w-0 touch-manipulation rounded-lg border border-outline bg-surface px-1 py-1.5 text-center focus-visible:outline-2 focus-visible:outline-accent"
  onpointerdown={(event) => {
    const button = event.currentTarget;
    activateFromPointer(event, () => open(button));
  }}
  onclick={(event) => {
    const button = event.currentTarget;
    activateFromKeyboardOrAssistiveClick(event, () => open(button));
  }}
>
  <div id={`live-number-${slot}`} role="group" aria-label={liveValueCatalog[field].name}>
    <LiveNumber {field} {sample} {units} {estimatedLevel} {currentLimits} />
  </div>
</button>
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
