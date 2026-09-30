<script lang="ts">
  import type { BatteryCellProfile } from '$lib/backend/contract';
  import { validSeriesCount, validParallelCount } from './autosave';
  let {
    seriesCount = $bindable(),
    parallelCount = $bindable(),
    batteryCellProfileId = $bindable(''),
    batteryMinimumCellVoltageV = $bindable(),
    profiles,
    saveProfile,
  }: {
    seriesCount?: number;
    parallelCount?: number;
    batteryCellProfileId?: string;
    batteryMinimumCellVoltageV?: number;
    profiles: BatteryCellProfile[];
    saveProfile: (patch: Record<string, unknown>) => void;
  } = $props();
  const validMinimum = (v: number) => Number.isFinite(v) && v >= 2.7 && v < 4.2;
</script>

<label class="text-sm sm:col-span-2">
  <span id="battery-cell-profile-label" class="mb-1 block text-muted">Battery cell profile</span>
  <select
    class="w-full touch-manipulation rounded-md border border-outline bg-elevated px-3 py-2"
    aria-labelledby="battery-cell-profile-label"
    bind:value={batteryCellProfileId}
    onchange={() => saveProfile({ batteryCellProfileId: batteryCellProfileId || null })}
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
    onblur={() => {
      if (batteryMinimumCellVoltageV === undefined) saveProfile({ batteryMinimumCellVoltageV: null });
      else if (validMinimum(batteryMinimumCellVoltageV)) saveProfile({ batteryMinimumCellVoltageV });
    }}
  />
  <span id="battery-estimate-help" class="mt-1 block text-xs text-muted">
    Defaults to 3.0 V. Changes the approximate displayed estimate, not controller protection settings.
  </span>
  {#if batteryMinimumCellVoltageV !== undefined && !validMinimum(batteryMinimumCellVoltageV)}
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
    bind:value={seriesCount}
    onblur={() => {
      if (seriesCount === undefined) saveProfile({ seriesCount: null });
      else if (validSeriesCount(seriesCount)) saveProfile({ seriesCount });
    }}
  />
  {#if seriesCount !== undefined && !validSeriesCount(seriesCount)}
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
    bind:value={parallelCount}
    onblur={() => {
      if (parallelCount === undefined) saveProfile({ parallelCount: null });
      else if (validParallelCount(parallelCount)) saveProfile({ parallelCount });
    }}
  />
  {#if parallelCount !== undefined && !validParallelCount(parallelCount)}
    <span role="alert" class="mt-1 block text-xs text-warning">Enter a whole count from 1 to 20.</span>
  {/if}
</label>
