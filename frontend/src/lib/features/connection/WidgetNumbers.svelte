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

  let { sample }: { sample: LiveSample | null | undefined } = $props();
  const slots = $derived(backendState.settings?.liveWidgetSlots ?? defaultLiveWidgetSlots);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const estimatedLevel = $derived(backendState.snapshot?.activeConnection?.batteryEstimate.level);
  let selectedSlot = $state<number | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let trigger: HTMLButtonElement | undefined;

  function open(slot: number, button: HTMLButtonElement) {
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

<div class="grid grid-cols-4 gap-1.5" aria-label="Live number widgets">
  {#each slots as field, index (index)}
    <button
      type="button"
      aria-label={`Choose value for box ${index + 1}: ${liveValueCatalog[field].name}`}
      aria-describedby={`live-number-${index}`}
      class="min-h-11 min-w-0 touch-manipulation rounded-lg border border-outline bg-surface px-1 py-1.5 text-center focus-visible:outline-2 focus-visible:outline-accent"
      onpointerdown={(event) => {
        const button = event.currentTarget;
        activateFromPointer(event, () => open(index, button));
      }}
      onclick={(event) => {
        const button = event.currentTarget;
        activateFromKeyboardOrAssistiveClick(event, () => open(index, button));
      }}
    >
      <div id={`live-number-${index}`} role="group" aria-label={liveValueCatalog[field].name}>
        <LiveNumber {field} {sample} {units} {estimatedLevel} />
      </div>
    </button>
  {/each}
</div>
{#if selectedSlot != null}
  <LiveValuePicker
    slot={selectedSlot}
    selected={slots[selectedSlot]}
    {sample}
    {units}
    {estimatedLevel}
    {busy}
    {error}
    onselect={select}
    onclose={close}
  />
{/if}
