<script lang="ts">
  import type { LiveSample } from '../../backend/contract';
  import { colorForTemperature } from './gauge-colors';
  import GaugeBar from './WidgetBar.svelte';

  let {
    sample,
  }: {
    sample: LiveSample | null | undefined;
  } = $props();

  const maximum = 100;
  const labelValues = Array.from({ length: 6 }, (_, index) => index * 20);

  function label(value: number | null | undefined) {
    return value == null ? '—' : `${value.toFixed(1)} °C`;
  }

  const readings = $derived([
    { key: 'controller', name: 'VESC', value: sample?.controllerTempC },
    { key: 'motor', name: 'Motor', value: sample?.motorTempC },
  ]);
</script>

<section class="flex h-full min-h-0 flex-col overflow-hidden" aria-labelledby="temperatures-title">
  <h2
    id="temperatures-title"
    class="mb-3 shrink-0 text-xs font-semibold tracking-[0.16em] text-muted uppercase tall:text-sm taller:text-base"
  >
    Temperatures
  </h2>

  <div class="flex min-h-0 flex-1 flex-col gap-3">
    {#each readings as reading (reading.key)}
      <GaugeBar
        value={reading.value}
        {maximum}
        label={reading.name}
        detail={label(reading.value)}
        ariaLabel={`${reading.name} temperature`}
        ariaValueText={reading.value == null ? 'Unavailable' : label(reading.value)}
        showTicks
        showLabels={reading.key === 'motor'}
        tickValues={labelValues}
        format={(value) => `${value}°`}
        heightClass="h-5"
        fillClass="bg-linear-to-r from-green-600 to-[var(--temperature-color)]"
        fillStyle={`--temperature-color: ${colorForTemperature(reading.value)}`}
      />
    {/each}
  </div>
</section>
