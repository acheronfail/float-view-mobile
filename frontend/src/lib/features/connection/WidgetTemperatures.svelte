<script lang="ts">
  import type { LiveSample } from '../../backend/contract';
  import { bmsMenuVisible } from './bms';
  import { liveValue } from './live-values';
  import type { BmsState } from '../../backend/contract';
  import { colorForTemperature } from './gauge-colors';
  import GaugeBar from './WidgetBar.svelte';

  let {
    sample,
    bms,
    nowMs,
  }: {
    sample: LiveSample | null | undefined;
    bms?: BmsState | null;
    nowMs: number;
  } = $props();

  const maximum = 100;
  const labelValues = Array.from({ length: 6 }, (_, index) => index * 20);

  function label(value: number | null | undefined) {
    return value == null ? '—' : `${value.toFixed(1)} °C`;
  }

  const batteryTemperature = $derived(
    liveValue('bmsHighestCellTempC', sample, 'metric', undefined, undefined, bms, Math.max(nowMs, Date.now())).value
  );
  const showBattery = $derived(bmsMenuVisible(bms) || batteryTemperature != null);

  const readings = $derived([
    { key: 'controller', name: 'VESC', value: sample?.controllerTempC },
    { key: 'motor', name: 'Motor', value: sample?.motorTempC },
    ...(showBattery ? [{ key: 'battery', name: 'Batt.', value: batteryTemperature }] : []),
  ]);
</script>

<section class="flex h-full min-h-0 flex-col overflow-hidden" aria-labelledby="temperatures-title">
  <h2
    id="temperatures-title"
    class="mb-3 shrink-0 text-xs font-semibold tracking-[0.16em] text-muted uppercase tall:text-sm taller:text-base"
  >
    Temperatures
  </h2>

  <div class={`flex h-[4.5rem] shrink-0 flex-col ${showBattery ? 'gap-0.5' : 'gap-3'}`}>
    {#each readings as reading, index (reading.key)}
      <GaugeBar
        value={reading.value}
        {maximum}
        label={reading.name}
        detail={label(reading.value)}
        ariaLabel={`${reading.key === 'battery' ? 'Battery' : reading.name} temperature`}
        ariaValueText={reading.value == null ? 'Unavailable' : label(reading.value)}
        showTicks
        showLabels={index === readings.length - 1}
        tickValues={labelValues}
        format={(value) => `${value}°`}
        compact={showBattery}
        heightClass={showBattery ? 'h-3' : 'h-5'}
        fillClass="bg-linear-to-r from-green-600 to-[var(--temperature-color)]"
        fillStyle={`--temperature-color: ${colorForTemperature(reading.value)}`}
      />
    {/each}
  </div>
</section>
