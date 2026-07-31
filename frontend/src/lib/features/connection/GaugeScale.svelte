<script lang="ts">
  let {
    values,
    minimum = 0,
    maximum,
    labels = false,
    format = (value: number) => String(value),
  }: {
    values: readonly number[];
    minimum?: number;
    maximum: number;
    labels?: boolean;
    format?: (value: number) => string;
  } = $props();

  function position(value: number) {
    if (maximum === minimum) return 0;
    return ((value - minimum) / (maximum - minimum)) * 100;
  }

  function anchor(value: number) {
    const offset = position(value);
    if (offset <= 0) return 'start';
    if (offset >= 100) return 'end';
    return 'middle';
  }
</script>

{#if labels}
  <svg
    class="block h-4 w-full overflow-visible text-[0.625rem] text-muted/70"
    width="100%"
    height="16"
    aria-hidden="true"
  >
    {#each values as value (value)}
      <text x={`${position(value)}%`} y="11" fill="currentColor" text-anchor={anchor(value)}>{format(value)}</text>
    {/each}
  </svg>
{:else}
  <svg
    class="pointer-events-none absolute inset-0 z-10 h-full w-full text-foreground/20"
    width="100%"
    height="100%"
    aria-hidden="true"
    preserveAspectRatio="none"
  >
    {#each values as value (value)}
      <line
        class="stroke-current stroke-1 [vector-effect:non-scaling-stroke]"
        x1={`${position(value)}%`}
        y1="0"
        x2={`${position(value)}%`}
        y2="100%"
      />
    {/each}
  </svg>
{/if}
