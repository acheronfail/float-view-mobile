<script lang="ts">
  import type { LiveSample } from '../../backend/contract';
  import { freshBmsSnapshot } from './bms';
  import { liveValue } from './live-values';
  import type { BmsState } from '../../backend/contract';
  import { colorForTemperature } from './gauge-colors';
  import GaugeBar from './WidgetBar.svelte';

  let {
    sample,
    bms,
    nowMs,
    idPrefix = 'temperatures',
    dashboard = false,
    expanded = false,
  }: {
    sample: LiveSample | null | undefined;
    bms?: BmsState | null;
    nowMs: number;
    idPrefix?: string;
    dashboard?: boolean;
    expanded?: boolean;
  } = $props();

  const maximum = 100;
  const labelValues = Array.from({ length: 6 }, (_, index) => index * 20);

  function label(value: number | null | undefined) {
    return value == null ? '—' : `${value.toFixed(1)} °C`;
  }

  const batteryTemperature = $derived(
    liveValue('bmsHighestCellTempC', sample, 'metric', undefined, bms, Math.max(nowMs, Date.now())).value
  );
  const circuitryTemperature = $derived(
    freshBmsSnapshot(bms, Math.max(nowMs, Date.now()))?.highestCircuitryTemperatureC ?? null
  );
  const showBattery = $derived(
    bms != null && bms.status !== 'not_found' && (bms.connection != null || bms.candidates.length > 0)
  );

  const readings = $derived([
    { key: 'controller', name: 'VESC', value: sample?.controllerTempC },
    { key: 'motor', name: 'Motor', value: sample?.motorTempC },
    ...(showBattery
      ? [
          { key: 'bms', name: 'BMS', value: circuitryTemperature },
          { key: 'battery', name: 'Cells', value: batteryTemperature },
        ]
      : []),
  ]);
</script>

{#if dashboard}
  <section class="dashboard-temperatures h-full min-h-0 w-full" aria-label="Temperatures">
    <div class="temperature-content" class:with-bms={showBattery} class:expanded>
      <h2 class="temperature-title live-widget-title">
        <span class="full-title">Temperatures °C</span><span class="short-title">Temps °C</span>
      </h2>
      <div class="temperature-readings">
        {#each readings as reading (reading.key)}
          <div class="temperature-reading">
            <p class="temperature-label font-semibold">
              <span class="text-muted">{reading.name}</span><span class="font-mono tabular-nums"
                >{reading.value == null ? '—' : reading.value.toFixed(1)}<span class="temperature-unit"> °C</span></span
              >
            </p>
            <GaugeBar
              value={reading.value}
              {maximum}
              ariaLabel={`${reading.name} temperature`}
              ariaValueText={reading.value == null ? 'Unavailable' : label(reading.value)}
              showTicks={false}
              heightClass="h-1"
              fillClass="bg-linear-to-r from-green-600 to-[var(--temperature-color)]"
              fillStyle={`--temperature-color: ${colorForTemperature(reading.value, reading.key === 'battery' ? 45 : 60)}`}
            />
          </div>
        {/each}
      </div>
    </div>
  </section>
{:else}
  <section class="live-temperatures flex flex-col" aria-labelledby={`${idPrefix}-title`}>
    <h2
      id={`${idPrefix}-title`}
      class="mb-3 shrink-0 text-xs font-semibold tracking-[0.16em] text-muted uppercase tall:text-sm taller:text-base"
    >
      Temperatures
    </h2>

    <div class={`flex shrink-0 flex-col ${showBattery ? 'min-h-[5.625rem] gap-0.5' : 'h-[4.5rem] gap-3'}`}>
      {#each readings as reading, index (reading.key)}
        <GaugeBar
          value={reading.value}
          {maximum}
          label={reading.name}
          detail={label(reading.value)}
          ariaLabel={`${reading.key === 'battery' ? 'Battery cells' : reading.key === 'bms' ? 'BMS circuitry' : reading.name} temperature`}
          ariaValueText={reading.value == null ? 'Unavailable' : label(reading.value)}
          showTicks
          showLabels={index === readings.length - 1}
          tickValues={labelValues}
          format={(value) => `${value}°`}
          compact={showBattery}
          heightClass={showBattery ? 'h-3' : 'h-5'}
          fillClass="bg-linear-to-r from-green-600 to-[var(--temperature-color)]"
          fillStyle={`--temperature-color: ${colorForTemperature(reading.value, reading.key === 'battery' ? 45 : 60)}`}
        />
      {/each}
    </div>
  </section>
{/if}

<style>
  .dashboard-temperatures {
    container: dashboard-temperatures / size;
  }
  .temperature-content {
    display: flex;
    height: 100%;
    min-height: 0;
    flex-direction: column;
    justify-content: center;
    gap: 0;
  }
  .temperature-title {
    font-size: 10px;
    line-height: 11px;
    overflow-wrap: anywhere;
  }
  .full-title {
    display: none;
  }
  @container dashboard-temperatures (min-width: 120px) {
    .full-title {
      display: inline;
    }
    .short-title {
      display: none;
    }
  }
  .temperature-reading :global([role='progressbar']) {
    height: 3px;
  }
  .temperature-readings {
    display: grid;
    gap: 0;
  }
  .temperature-label {
    display: flex;
    justify-content: space-between;
    gap: 0;
    font-size: min(9px, 12cqi);
    line-height: 11px;
  }
  .temperature-label span {
    white-space: nowrap;
  }
  .with-bms .temperature-unit {
    display: none;
  }
  .with-bms .temperature-readings {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .with-bms .temperature-label {
    flex-direction: row;
    gap: 0;
    font-size: 6.5px;
    line-height: 9px;
  }
  @container dashboard-temperatures (min-height: 90px) {
    .with-bms .temperature-unit {
      display: inline;
    }
    .temperature-title,
    .with-bms .temperature-title {
      display: block;
      font-size: 10px;
      line-height: 14px;
    }
    .with-bms .temperature-readings {
      grid-template-columns: 1fr;
    }
    .with-bms .temperature-label {
      flex-direction: row;
      font-size: min(9px, 12cqi);
      line-height: 11px;
    }
    .temperature-content {
      gap: 4px;
    }
    .temperature-readings {
      gap: 4px;
    }
  }
  @container dashboard-temperatures (min-width: 240px) and (max-height: 89px) {
    .temperature-readings,
    .with-bms .temperature-readings {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      column-gap: 12px;
    }
    .with-bms .temperature-label {
      flex-direction: row;
    }
  }
  @container dashboard-temperatures (min-width: 120px) and (min-height: 65px) {
    .temperature-content.expanded {
      gap: 4px;
      justify-content: space-evenly;
    }
    .expanded .temperature-readings {
      grid-template-columns: 1fr;
      gap: 8px;
    }
    .expanded .temperature-label {
      font-size: clamp(12px, min(10cqh, 6cqi), 24px);
      line-height: 1.25;
    }
    .expanded .temperature-reading :global([role='progressbar']) {
      height: clamp(8px, 10cqh, 20px);
    }
    .expanded.with-bms .temperature-readings {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 4px 8px;
    }
    .expanded.with-bms .temperature-label {
      flex-direction: column;
      font-size: 10px;
      line-height: 12px;
    }
    .expanded.with-bms .temperature-unit {
      display: inline;
    }
    .expanded.with-bms .temperature-reading :global([role='progressbar']) {
      height: 6px;
    }
  }
  @container dashboard-temperatures (min-width: 160px) and (min-height: 140px) {
    .temperature-content.expanded {
      gap: 8px;
    }
    .expanded.with-bms .temperature-readings {
      gap: 8px 12px;
    }
    .expanded.with-bms .temperature-label {
      font-size: clamp(12px, min(7cqh, 4cqi), 20px);
      line-height: 1.25;
    }
    .expanded.with-bms .temperature-reading :global([role='progressbar']) {
      height: clamp(8px, 6cqh, 16px);
    }
  }

  @media (max-height: 40rem) {
    .live-temperatures h2 {
      margin-bottom: 0.25rem;
    }
  }
</style>
