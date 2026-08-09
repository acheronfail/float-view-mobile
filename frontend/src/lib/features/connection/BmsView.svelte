<script lang="ts">
  import { onMount } from 'svelte';
  import type { BmsState, UnitPreference } from '$lib/backend/contract';
  import { formatTemperatureC } from '$lib/features/measurements/format';
  import { bmsFreshness, bmsRouteLabel } from './bms';

  let { bms, units }: { bms: BmsState; units: UnitPreference } = $props();
  let now = $state(Date.now());
  const snapshot = $derived(bms.latestSnapshot);
  const candidates = $derived(
    [...bms.candidates].sort((left, right) => {
      if (left.connection.kind === 'direct') return right.connection.kind === 'direct' ? 0 : -1;
      if (right.connection.kind === 'direct') return 1;
      return left.connection.canId - right.connection.canId;
    })
  );

  onMount(() => {
    const timer = window.setInterval(() => (now = Date.now()), 1_000);
    return () => window.clearInterval(timer);
  });

  const number = (value: number | null | undefined, suffix: string, digits = 2) =>
    value == null ? '—' : `${value.toFixed(digits)} ${suffix}`;
</script>

<div class="grid gap-4 text-sm">
  <section class="grid grid-cols-2 gap-2 rounded-lg border border-outline bg-elevated p-3" aria-label="BMS status">
    <div>
      <span class="block text-muted">Status</span><strong class="capitalize">{bms.status.replace('_', ' ')}</strong>
    </div>
    <div><span class="block text-muted">Route</span><strong>{bmsRouteLabel(bms.connection)}</strong></div>
    <div class="col-span-2"><span class="block text-muted">UUID</span><code>{bms.uuid ?? 'Unavailable'}</code></div>
    <div class="col-span-2">
      <span class="block text-muted">Freshness</span>
      <strong>{snapshot ? bmsFreshness(snapshot.observedAt, now) : 'No values received'}</strong>
    </div>
  </section>

  {#if bms.status === 'ambiguous'}
    <section class="rounded-lg border border-warning/50 bg-warning/10 p-3" aria-label="Ambiguous BMS candidates">
      <h3 class="font-semibold">Choose a BMS route</h3>
      <p class="mt-1 text-muted">
        Set Preferred BMS CAN ID in Board configuration. Controller selection is unaffected.
      </p>
      <ul class="mt-2 grid gap-1">
        {#each candidates as candidate (bmsRouteLabel(candidate.connection))}
          <li>
            {bmsRouteLabel(candidate.connection)} · {candidate.hardwareName ?? 'VBMS'} · {candidate.vescVersion} ·
            <code>{candidate.uuid ?? 'UUID unavailable'}</code>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if bms.status === 'unavailable'}
    <p role="status" class="rounded-lg border border-warning/50 bg-warning/10 p-3">
      BMS values are unavailable. Any values below are retained from their original observation time.
    </p>
  {/if}

  {#if snapshot}
    <section aria-labelledby="bms-summary-title">
      <h3 id="bms-summary-title" class="mb-2 font-semibold">Summary</h3>
      <dl class="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <div class="rounded border border-outline p-2">
          <dt class="text-muted">Pack</dt>
          <dd>{number(snapshot.totalVoltageV, 'V')}</dd>
        </div>
        <div class="rounded border border-outline p-2">
          <dt class="text-muted">Charge target</dt>
          <dd>{number(snapshot.chargeVoltageV, 'V')}</dd>
        </div>
        <div class="rounded border border-outline p-2">
          <dt class="text-muted">Input</dt>
          <dd>{number(snapshot.inputCurrentA, 'A')}</dd>
        </div>
        <div class="rounded border border-outline p-2">
          <dt class="text-muted">SoC / SoH</dt>
          <dd>{number(snapshot.stateOfCharge * 100, '%', 1)} / {number(snapshot.stateOfHealth * 100, '%', 1)}</dd>
        </div>
      </dl>
    </section>

    <section aria-labelledby="bms-cells-title">
      <h3 id="bms-cells-title" class="mb-2 font-semibold">Cells and balancing</h3>
      <ol class="grid grid-cols-2 gap-1 sm:grid-cols-4">
        {#each snapshot.cellVoltagesV as voltage, index (index)}
          <li class="flex justify-between rounded border border-outline px-2 py-1">
            <span>Cell {index + 1}</span><span
              >{voltage.toFixed(3)} V {snapshot.balancing[index] ? '· balancing' : ''}</span
            >
          </li>
        {/each}
      </ol>
    </section>

    <section aria-labelledby="bms-temperature-title">
      <h3 id="bms-temperature-title" class="mb-2 font-semibold">Temperatures</h3>
      <ul class="grid grid-cols-2 gap-1 sm:grid-cols-3">
        {#each snapshot.temperaturesC as temperature, index (index)}
          <li class="rounded border border-outline px-2 py-1">
            Sensor {index + 1}: {formatTemperatureC(temperature, units)}
          </li>
        {/each}
        <li class="rounded border border-outline px-2 py-1">
          IC: {formatTemperatureC(snapshot.icTemperatureC, units)}
        </li>
        <li class="rounded border border-outline px-2 py-1">
          Highest cell: {formatTemperatureC(snapshot.highestCellTemperatureC, units)}
        </li>
        <li class="rounded border border-outline px-2 py-1">
          Humidity sensor: {formatTemperatureC(snapshot.humiditySensorTemperatureC, units)}
        </li>
      </ul>
    </section>

    <section aria-labelledby="bms-detail-title">
      <h3 id="bms-detail-title" class="mb-2 font-semibold">Environment and counters</h3>
      <dl class="grid grid-cols-2 gap-x-4 gap-y-1">
        <dt class="text-muted">Humidity</dt>
        <dd>{number(snapshot.humidityPercent, '%', 1)}</dd>
        <dt class="text-muted">Pressure</dt>
        <dd>{number(snapshot.pressurePa / 1000, 'kPa', 1)}</dd>
        <dt class="text-muted">Session</dt>
        <dd>{number(snapshot.sessionAmpHoursAh, 'Ah')} · {number(snapshot.sessionWattHoursWh, 'Wh')}</dd>
        <dt class="text-muted">Lifetime charge</dt>
        <dd>{number(snapshot.lifetimeChargeAmpHoursAh, 'Ah')} · {number(snapshot.lifetimeChargeWattHoursWh, 'Wh')}</dd>
        <dt class="text-muted">Lifetime discharge</dt>
        <dd>
          {number(snapshot.lifetimeDischargeAmpHoursAh, 'Ah')} · {number(snapshot.lifetimeDischargeWattHoursWh, 'Wh')}
        </dd>
        <dt class="text-muted">Reported CAN ID</dt>
        <dd>{snapshot.reportedCanId}</dd>
        <dt class="text-muted">Data version</dt>
        <dd>{snapshot.dataVersion ?? 'Legacy'}</dd>
        <dt class="text-muted">BMS status</dt>
        <dd>{snapshot.status ?? '—'}</dd>
      </dl>
    </section>
  {:else if bms.status !== 'ambiguous'}
    <p class="text-muted">BMS values will appear after the selected route responds.</p>
  {/if}
</div>
