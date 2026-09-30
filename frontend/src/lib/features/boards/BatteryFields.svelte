<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import type { BatteryCellProfile, BatteryPreset } from '$lib/backend/contract';
  import { validSeriesCount, validParallelCount } from './autosave';
  let {
    seriesCount = $bindable(),
    parallelCount = $bindable(),
    batteryCellProfileId = $bindable(''),
    batteryMinimumCellVoltageV = $bindable(),
    batteryPresetId = $bindable(''),
    batteryCapacityAh = $bindable(),
    presets,
    profiles,
    saveProfile,
  }: {
    seriesCount?: number;
    parallelCount?: number;
    batteryCellProfileId?: string;
    batteryMinimumCellVoltageV?: number;
    batteryPresetId?: string;
    batteryCapacityAh?: number;
    presets: BatteryPreset[];
    profiles: BatteryCellProfile[];
    saveProfile: (patch: Record<string, unknown>) => void;
  } = $props();
  const validMinimum = (v: number) => Number.isFinite(v) && v >= 2.7 && v < 4.2;
  const validCapacity = (v: number) => Number.isFinite(v) && v >= 0.1 && v <= 200;
  let mode = $state<'basic' | 'advanced'>(
    untrack(() =>
      batteryPresetId ||
      (seriesCount == null &&
        parallelCount == null &&
        !batteryCellProfileId &&
        batteryCapacityAh == null &&
        batteryMinimumCellVoltageV == null)
        ? 'basic'
        : 'advanced'
    )
  );
  let switching = false;
  let lastSeries = untrack(() => seriesCount);
  let lastParallel = untrack(() => parallelCount);
  const groups = $derived([...new Set(presets.map((p) => p.manufacturer))]);
  const namedPreset = $derived(presets.find((p) => p.id === batteryPresetId));
  const matchingPreset = $derived(
    namedPreset &&
      namedPreset.seriesCount === seriesCount &&
      namedPreset.parallelCount === parallelCount &&
      namedPreset.batteryCellProfileId === batteryCellProfileId
      ? namedPreset
      : undefined
  );
  const selected = $derived(
    matchingPreset?.id ?? (seriesCount != null || parallelCount != null || batteryCellProfileId ? 'custom' : '')
  );
  async function switchMode(next: 'basic' | 'advanced') {
    switching = true;
    mode = next;
    await tick();
    switching = false;
  }
  function skipBlur(event: FocusEvent) {
    return (
      switching || (event.relatedTarget instanceof Element && !!event.relatedTarget.closest('[data-battery-mode]'))
    );
  }
  function selectPreset(id: string) {
    if (id === 'custom') {
      void switchMode('advanced');
      return;
    }
    const p = presets.find((p) => p.id === id);
    if (!p) return;
    batteryPresetId = p.id;
    seriesCount = lastSeries = p.seriesCount;
    parallelCount = lastParallel = p.parallelCount;
    batteryCellProfileId = p.batteryCellProfileId;
    batteryCapacityAh = p.nominalCapacityAh ?? undefined;
    batteryMinimumCellVoltageV = undefined;
    saveProfile({ batteryPresetId: p.id });
  }
  function saveManual(patch: Record<string, unknown>) {
    if (
      ('seriesCount' in patch && seriesCount !== lastSeries) ||
      ('parallelCount' in patch && parallelCount !== lastParallel)
    )
      batteryCapacityAh = undefined;
    if ('seriesCount' in patch) lastSeries = seriesCount;
    if ('parallelCount' in patch) lastParallel = parallelCount;
    if (!matchingPreset) batteryPresetId = '';
    saveProfile(patch);
  }
</script>

<div role="group" aria-label="Battery setup mode" class="flex gap-1 sm:col-span-2">
  {#each ['basic', 'advanced'] as choice (choice)}
    <button
      type="button"
      data-battery-mode
      aria-pressed={mode === choice}
      class="touch-manipulation rounded-md border border-outline px-3 py-2 text-sm aria-pressed:bg-elevated"
      onpointerdown={(event) => activateFromPointer(event, () => switchMode(choice as 'basic' | 'advanced'))}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => switchMode(choice as 'basic' | 'advanced'))}
    >
      {choice === 'basic' ? 'Basic' : 'Advanced'}
    </button>
  {/each}
</div>
{#if mode === 'basic'}
  <label class="text-sm sm:col-span-2">
    <span class="mb-1 block text-muted">Battery</span>
    <select
      class="w-full touch-manipulation rounded-md border border-outline bg-elevated px-3 py-2"
      value={selected}
      onchange={(event) => selectPreset(event.currentTarget.value)}
    >
      <option value="" disabled>Choose your battery</option>
      {#each groups as manufacturer (manufacturer)}
        <optgroup label={manufacturer}>
          {#each presets.filter((p) => p.manufacturer === manufacturer) as preset (preset.id)}
            <option value={preset.id}>{preset.label}</option>
          {/each}
        </optgroup>
      {/each}
      <option value="custom">Custom / not listed</option>
    </select>
  </label>
  <p class="text-sm text-muted sm:col-span-2" aria-label="Battery configuration summary">
    {seriesCount ?? '—'}S{parallelCount ?? '—'}P
    {#if matchingPreset}
      · {matchingPreset.cellLabel}{/if}
    · {batteryCapacityAh == null
      ? 'Capacity not set'
      : `${batteryCapacityAh} Ah${matchingPreset?.nominalCapacityAh === batteryCapacityAh ? ' nominal' : ''}`}
  </p>
{:else}
  {#if matchingPreset}
    <button
      type="button"
      class="touch-manipulation rounded-md border border-outline px-3 py-2 text-sm sm:col-span-2"
      onpointerdown={(event) => activateFromPointer(event, () => selectPreset(matchingPreset!.id))}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => selectPreset(matchingPreset!.id))}
    >
      Reset to battery defaults
    </button>
  {/if}

  <label class="text-sm sm:col-span-2">
    <span id="battery-cell-profile-label" class="mb-1 block text-muted">Battery cell profile</span>
    <select
      class="w-full touch-manipulation rounded-md border border-outline bg-elevated px-3 py-2"
      aria-labelledby="battery-cell-profile-label"
      bind:value={batteryCellProfileId}
      onchange={() => saveManual({ batteryCellProfileId: batteryCellProfileId || null })}
    >
      <option value="">Not configured</option>
      {#each profiles as profile (profile.id)}
        <option value={profile.id}>{profile.label}</option>
      {/each}
    </select>
  </label>
  <label class="text-sm sm:col-span-2">
    <span id="battery-empty-voltage-label" class="mb-1 block text-muted">Display empty voltage per cell</span>
    <input
      class="w-full touch-manipulation rounded-md border border-outline bg-elevated px-3 py-2"
      type="number"
      min="2.7"
      max="4.199"
      step="any"
      inputmode="decimal"
      placeholder="3.0"
      aria-labelledby="battery-empty-voltage-label"
      bind:value={batteryMinimumCellVoltageV}
      aria-describedby="battery-estimate-help"
      onblur={(event) => {
        if (skipBlur(event)) return;
        if (batteryMinimumCellVoltageV == null) saveManual({ batteryMinimumCellVoltageV: null });
        else if (validMinimum(batteryMinimumCellVoltageV)) saveManual({ batteryMinimumCellVoltageV });
      }}
    />
    <span id="battery-estimate-help" class="mt-1 block text-xs text-muted">
      Defaults to 3.0 V. Changes the approximate displayed estimate, not controller protection settings.
    </span>
    {#if batteryMinimumCellVoltageV != null && !validMinimum(batteryMinimumCellVoltageV)}
      <span role="alert" class="mt-1 block text-xs text-warning">Enter a voltage from 2.7 V to below 4.2 V.</span>
    {/if}
  </label>
  <label class="text-sm">
    <span class="mb-1 block text-muted">Battery series cells (S)</span>
    <input
      class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
      type="number"
      min="1"
      max="36"
      inputmode="numeric"
      aria-label="Battery series cells (S)"
      bind:value={seriesCount}
      onblur={(event) => {
        if (skipBlur(event)) return;
        if (seriesCount == null) saveManual({ seriesCount: null });
        else if (validSeriesCount(seriesCount)) saveManual({ seriesCount });
      }}
    />
    {#if seriesCount != null && !validSeriesCount(seriesCount)}
      <span role="alert" class="mt-1 block text-xs text-warning">Enter a whole count from 1 to 36.</span>
    {/if}
  </label>
  <label class="text-sm">
    <span class="mb-1 block text-muted">Battery parallel cells (P)</span>
    <input
      class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
      type="number"
      min="1"
      max="20"
      inputmode="numeric"
      aria-label="Battery parallel cells (P)"
      bind:value={parallelCount}
      onblur={(event) => {
        if (skipBlur(event)) return;
        if (parallelCount == null) saveManual({ parallelCount: null });
        else if (validParallelCount(parallelCount)) saveManual({ parallelCount });
      }}
    />
    {#if parallelCount != null && !validParallelCount(parallelCount)}
      <span role="alert" class="mt-1 block text-xs text-warning">Enter a whole count from 1 to 20.</span>
    {/if}
  </label>

  <label class="text-sm sm:col-span-2">
    <span class="mb-1 block text-muted">Pack capacity (Ah, optional)</span>
    <input
      class="w-full touch-manipulation rounded-md border border-outline bg-elevated px-3 py-2"
      type="number"
      min="0.1"
      max="200"
      step="any"
      inputmode="decimal"
      aria-label="Pack capacity (Ah, optional)"
      bind:value={batteryCapacityAh}
      aria-describedby="battery-capacity-help"
      onblur={(event) => {
        if (skipBlur(event)) return;
        if (batteryCapacityAh == null) saveManual({ batteryCapacityAh: null });
        else if (validCapacity(batteryCapacityAh)) saveManual({ batteryCapacityAh });
      }}
    />
    <span id="battery-capacity-help" class="mt-1 block text-xs text-muted">
      Full pack capacity: two parallel 4.5 Ah cells give 9 Ah. Series count does not multiply Ah. Leave blank if
      unknown.
    </span>
    {#if batteryCapacityAh != null && !validCapacity(batteryCapacityAh)}
      <span role="alert" class="mt-1 block text-xs text-warning">Enter a capacity from 0.1 to 200 Ah.</span>
    {/if}
  </label>
{/if}
