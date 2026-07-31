<script lang="ts">
  import type { LiveSample } from '../../backend/contract';
  import GaugeScale from './GaugeScale.svelte';

  let {
    sample,
  }: {
    sample: LiveSample | null | undefined;
  } = $props();

  const green = [22, 163, 74];
  const orange = [234, 88, 12];
  const red = [220, 38, 38];
  const maximum = 70;
  const tickValues = Array.from({ length: 15 }, (_, index) => index * 5);
  const labelValues = Array.from({ length: 8 }, (_, index) => index * 10);

  function interpolate(start: number[], end: number[], amount: number) {
    return start.map((value, index) => Math.round(value + (end[index] - value) * amount));
  }

  function rgb(color: number[]) {
    return `rgb(${color.join(', ')})`;
  }

  function colorForTemperature(value: number | null | undefined) {
    if (value == null || value <= 30) return rgb(green);
    if (value >= 60) return rgb(red);
    if (value >= 45) return rgb(interpolate(orange, red, (value - 45) / 15));
    return rgb(interpolate(green, orange, (value - 30) / 15));
  }

  function progress(value: number | null | undefined) {
    return value == null ? 0 : (Math.min(Math.max(value, 0), maximum) / maximum) * 100;
  }

  function label(value: number | null | undefined) {
    return value == null ? '—' : `${value.toFixed(1)} °C`;
  }

  const readings = $derived([
    { key: 'controller', name: 'VESC', value: sample?.controllerTempC },
    { key: 'motor', name: 'Motor', value: sample?.motorTempC },
  ]);
</script>

<section class="rounded-2xl border border-outline bg-surface p-3 sm:p-5" aria-labelledby="temperatures-title">
  <h2 id="temperatures-title" class="mb-3 text-xs font-semibold tracking-[0.16em] text-muted uppercase">
    Temperatures
  </h2>

  <div class="space-y-3">
    {#each readings as reading (reading.key)}
      <div class="grid grid-cols-[2.5rem_minmax(0,1fr)_4.5rem] items-center gap-x-3">
        <span class="w-[2.5rem] shrink-0 text-xs font-semibold text-muted uppercase">{reading.name}</span>
        <div
          class="relative h-4 min-w-0 flex-1 overflow-hidden rounded-full bg-elevated"
          role="progressbar"
          aria-label={`${reading.name} temperature`}
          aria-valuemin="0"
          aria-valuemax={maximum}
          aria-valuenow={reading.value ?? undefined}
          aria-valuetext={reading.value == null ? 'Unavailable' : label(reading.value)}
        >
          <GaugeScale values={tickValues} {maximum} />
          <div
            class="h-full rounded-full bg-linear-to-r from-green-600 to-[var(--temperature-color)] transition-[width,background-color] duration-500"
            style={`width: ${progress(reading.value)}%; --temperature-color: ${colorForTemperature(reading.value)}`}
          ></div>
        </div>
        <output class="w-[4.5rem] shrink-0 text-right font-mono text-sm font-semibold tabular-nums">
          {label(reading.value)}
        </output>
      </div>
    {/each}
  </div>

  <div class="mt-1 grid grid-cols-[2.5rem_minmax(0,1fr)_4.5rem] gap-x-3">
    <div class="col-start-2 min-w-0">
      <GaugeScale values={labelValues} {maximum} labels format={(value) => `${value}°`} />
    </div>
  </div>
</section>
