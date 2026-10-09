<script lang="ts">
  import SegmentedControl from '$lib/ui/SegmentedControl.svelte';
  import type { SessionRideMarker } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  let {
    markers,
    selected,
    onselect,
  }: { markers: SessionRideMarker[]; selected: number | null; onselect: (marker: SessionRideMarker) => void } =
    $props();
  const index = $derived(markers.findIndex((m) => m.sequence === selected));
  function move(direction: number) {
    const marker = markers[index < 0 ? 0 : index + direction];
    if (marker) onselect(marker);
  }
  function label(marker: SessionRideMarker, i: number) {
    return `Marker ${i + 1} · ${(marker.elapsedMs / 1000).toFixed(1)}s${marker.location ? '' : ' · No location'}`;
  }
</script>

{#if markers.length}
  <div class="flex items-center gap-2 rounded-lg border border-outline bg-elevated p-2" aria-label="Ride markers">
    <button
      type="button"
      aria-label="Previous marker"
      disabled={index <= 0}
      class="min-h-11 touch-manipulation rounded px-2 disabled:opacity-40"
      onpointerdown={(e) => activateFromPointer(e, () => move(-1))}
      onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => move(-1))}>←</button
    >
    <div class="min-w-0 flex-1 text-sm">
      <p>Markers ({markers.length})</p>
      <SegmentedControl
        label="Select ride marker"
        value={selected ?? ''}
        options={[
          { value: '', label: 'Select a marker', disabled: true },
          ...markers.map((marker, i) => ({ value: marker.sequence, label: label(marker, i) })),
        ]}
        onchange={(value) => {
          const marker = markers.find((m) => m.sequence === value);
          if (marker) onselect(marker);
        }}
      />
    </div>
    <button
      type="button"
      aria-label="Next marker"
      disabled={index === markers.length - 1}
      class="min-h-11 touch-manipulation rounded px-2 disabled:opacity-40"
      onpointerdown={(e) => activateFromPointer(e, () => move(1))}
      onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => move(1))}>→</button
    >
  </div>
{/if}
