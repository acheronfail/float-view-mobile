<script lang="ts">
  let {
    value,
    maximum,
    ariaLabel,
    ariaValueText,
    label,
    detail,
    detailClass = '',
    compact = false,
    tickValues = [],
    minimum = 0,
    showTicks = true,
    showLabels = false,
    tickSpacing = 32,
    heightClass = 'h-4',
    fillClass = '',
    fillStyle = '',
    format = (tickValue: number) => String(tickValue),
  }: {
    value: number | null | undefined;
    maximum: number;
    ariaLabel: string;
    ariaValueText: string;
    label?: string;
    detail?: string;
    detailClass?: string;
    compact?: boolean;
    tickValues?: readonly number[];
    minimum?: number;
    showTicks?: boolean;
    showLabels?: boolean;
    tickSpacing?: number;
    heightClass?: string;
    fillClass?: string;
    fillStyle?: string;
    format?: (value: number) => string;
  } = $props();

  const progress = $derived(
    value == null || maximum <= minimum
      ? 0
      : ((Math.min(Math.max(value, minimum), maximum) - minimum) / (maximum - minimum)) * 100
  );

  function position(tickValue: number) {
    if (maximum === minimum) return 0;
    return ((tickValue - minimum) / (maximum - minimum)) * 100;
  }

  function anchor(tickValue: number) {
    const offset = position(tickValue);
    if (offset <= 0) return 'start';
    if (offset >= 100) return 'end';
    return 'middle';
  }
</script>

<div
  class={`grid min-h-0 ${label ? 'grid-cols-[2.5rem_minmax(0,1fr)_4.5rem]' : 'grid-cols-[minmax(0,1fr)_auto]'} ${
    showLabels ? 'grid-rows-[minmax(0,1fr)_auto]' : ''
  } items-center gap-x-3`}
>
  {#if label}
    <span class="w-[2.5rem] shrink-0 text-xs font-semibold text-muted uppercase">{label}</span>
  {/if}

  <div class={label ? 'col-start-2 min-w-0' : 'min-w-0'}>
    <div
      class={`relative ${heightClass} min-w-0 overflow-hidden rounded-full bg-elevated`}
      role="progressbar"
      aria-label={ariaLabel}
      aria-valuemin={minimum}
      aria-valuemax={maximum}
      aria-valuenow={value ?? undefined}
      aria-valuetext={ariaValueText}
    >
      {#if showTicks}
        <div
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 z-10 text-foreground/20"
          style={`background-image: repeating-linear-gradient(to right, currentColor 0, currentColor 1px, transparent 1px, transparent ${tickSpacing}px)`}
        ></div>
      {/if}
      <div
        class={`relative h-full rounded-full transition-[width,background-color] duration-500 ${fillClass}`}
        style={`width: ${progress}%; ${fillStyle}`}
      ></div>
    </div>
  </div>

  {#if detail != null}
    <output
      class={`${label ? 'w-[4.5rem]' : 'min-w-[2.5rem]'} shrink-0 text-right font-mono ${compact ? 'text-xs leading-4' : 'text-sm'} font-semibold tabular-nums ${detailClass}`}
    >
      {detail}
    </output>
  {/if}

  {#if showLabels}
    <div class={`mt-1 min-w-0 ${label ? 'col-start-2' : 'col-start-1'}`}>
      <svg
        class="block h-4 w-full overflow-visible text-[0.625rem] text-muted/70"
        width="100%"
        height="16"
        aria-hidden="true"
      >
        {#each tickValues as tickValue (tickValue)}
          <text x={`${position(tickValue)}%`} y="11" fill="currentColor" text-anchor={anchor(tickValue)}>
            {format(tickValue)}
          </text>
        {/each}
      </svg>
    </div>
  {/if}
</div>
