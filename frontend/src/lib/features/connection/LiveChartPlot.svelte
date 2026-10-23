<script lang="ts">
  import '$lib/ui/telemetry-chart.css';
  import { liveChartGeometry, liveChartColor, type LiveChartField } from './live-charts';
  import type { LiveHistory } from './live-history';
  import type { UnitPreference } from '$lib/backend/contract';
  let {
    history,
    fields,
    units,
    title,
    rawReadings = false,
    digits = 0,
    height,
    compact = false,
    inheritColor = false,
    dashboard = false,
    sparkline = false,
  }: {
    history: LiveHistory;
    fields: LiveChartField[];
    units: UnitPreference;
    title: string;
    rawReadings?: boolean;
    digits?: number;
    height?: string;
    compact?: boolean;
    inheritColor?: boolean;
    dashboard?: boolean;
    sparkline?: boolean;
  } = $props();
  const geometry = $derived(liveChartGeometry(history.points, fields, units, history.now, { rawReadings }));
  function axisLabel(value: number) {
    const span = Math.max(geometry.high - geometry.low, 0.000001);
    const precision = Math.min(6, Math.max(digits, Math.ceil(-Math.log10(span / 2)), 0));
    return Math.abs(value) >= 10000 || (value !== 0 && Math.abs(value) < 0.001)
      ? value.toExponential(1).replace('e+', 'e')
      : String(Number(value.toFixed(precision)));
  }
</script>

{#snippet plot()}
  <svg
    class="block h-full w-full overflow-hidden"
    viewBox="0 0 1000 100"
    preserveAspectRatio="none"
    role="img"
    aria-label={`${title}, rolling 30-second chart${dashboard ? `, range ${axisLabel(geometry.low)} to ${axisLabel(geometry.high)}` : ''}`}
  >
    {#each [0, 50, 100] as y (y)}<line
        x1="0"
        x2="1000"
        y1={y}
        y2={y}
        stroke="var(--color-outline)"
        vector-effect="non-scaling-stroke"
      />{/each}
    {#each [0, 250, 500, 750, 1000] as x (x)}<line
        x1={x}
        x2={x}
        y1="0"
        y2="100"
        stroke="var(--color-outline)"
        stroke-dasharray="2 5"
        vector-effect="non-scaling-stroke"
      />{/each}
    {#each geometry.paths as path, index (fields[index])}<path
        class="telemetry-chart-trace"
        d={path}
        fill="none"
        stroke={inheritColor ? 'currentColor' : liveChartColor(fields[index], index)}
        stroke-linejoin="round"
        vector-effect="non-scaling-stroke"
      />{/each}
  </svg>
{/snippet}

{#if dashboard}
  <div class="dashboard-chart flex h-full min-h-0 flex-col" class:sparkline>
    <div class="chart-body flex min-h-0 flex-1">
      <div
        class="chart-y-axis shrink-0 flex-col justify-between text-right font-mono text-[10px] leading-3 text-muted"
        aria-hidden="true"
      >
        <span>{axisLabel(geometry.high)}</span><span class="chart-middle"
          >{axisLabel((geometry.high + geometry.low) / 2)}</span
        ><span>{axisLabel(geometry.low)}</span>
      </div>
      <div class="telemetry-chart-frame relative min-h-0 min-w-0 flex-1" style:height="100%">{@render plot()}</div>
    </div>
    <div
      class="chart-time-axis mt-1 flex shrink-0 justify-between font-mono text-[10px] leading-3 text-muted"
      aria-hidden="true"
    >
      <span>−30s</span><span class="chart-middle-time">−15s</span><span>Now</span>
    </div>
  </div>
{:else}
  <div class="telemetry-chart-frame relative" style:height>
    <div
      class={`pointer-events-none absolute inset-y-0 z-10 flex flex-col items-start justify-between font-mono ${inheritColor ? '' : 'text-muted'} ${compact ? 'left-0 text-[7px]' : 'left-3 text-[10px]'}`}
    >
      <span class="bg-surface/80 px-1">{geometry.high.toFixed(digits)}</span>{#if !compact}<span
          class="bg-surface/80 px-1">{((geometry.high + geometry.low) / 2).toFixed(digits)}</span
        ><span class="bg-surface/80 px-1">{geometry.low.toFixed(digits)}</span>{/if}
    </div>
    {@render plot()}
  </div>
  <div
    class={`flex justify-between font-mono ${inheritColor ? '' : 'text-muted'} ${compact ? 'text-[7px] leading-3' : 'mt-1 px-4 text-[10px]'}`}
  >
    <span>−30s</span>{#if !compact}<span>−15s</span>{/if}<span>Now</span>
  </div>
{/if}

<style>
  .chart-y-axis {
    display: none;
    width: 3.5rem;
    padding-right: 8px;
  }
  .chart-middle,
  .chart-middle-time {
    display: none;
  }
  .sparkline .chart-time-axis,
  .sparkline .chart-y-axis {
    display: none;
  }
  .dashboard-chart :global(.telemetry-chart-trace) {
    stroke-width: 2;
  }
  @container custom-widget (min-width: 140px) and (min-height: 90px) {
    .dashboard-chart:not(.sparkline) .chart-y-axis {
      display: flex;
    }
    .dashboard-chart:not(.sparkline) .chart-time-axis {
      padding-left: 3.5rem;
    }
  }
  @container custom-widget (min-height: 170px) {
    .chart-middle {
      display: inline;
    }
  }
  @container custom-widget (min-width: 240px) {
    .chart-middle-time {
      display: inline;
    }
  }
</style>
