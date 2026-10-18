<script lang="ts">
  import { defaultBoardAppearance, type BoardAppearance } from '$lib/backend/contract';
  import { appearanceShapes } from './appearance';
  let {
    appearance,
    label = 'Board appearance',
    class: className = '',
  }: {
    appearance?: BoardAppearance | null;
    label?: string;
    class?: string;
  } = $props();
  const id = $props.id();
  const value = $derived(appearance ?? defaultBoardAppearance);
  const shapes = $derived(appearanceShapes(value));
  const sections = [
    ['left', 'top', 'leftTop'],
    ['left', 'bottom', 'leftBottom'],
    ['right', 'top', 'rightTop'],
    ['right', 'bottom', 'rightBottom'],
  ] as const;
</script>

<svg viewBox="0 16 480 196" role="img" aria-label={label} class={className} data-board-icon>
  <defs>
    <clipPath id={`${id}-top`}><polygon points={shapes.top} /></clipPath>
    <clipPath id={`${id}-bottom`}><polygon points={shapes.bottom} /></clipPath>
    <clipPath id={`${id}-left`}><rect x="0" y="0" width="240" height="248" /></clipPath>
    <clipPath id={`${id}-right`}><rect x="240" y="0" width="240" height="248" /></clipPath>
    <mask id={`${id}-wheel`} maskUnits="userSpaceOnUse" x="150" y="34" width="180" height="180">
      <rect x="150" y="34" width="180" height="180" fill="white" />
      <polyline
        points={shapes.rail}
        fill="none"
        stroke="black"
        stroke-width="38"
        stroke-linejoin="round"
        stroke-linecap="round"
      />
    </mask>
  </defs>
  <g mask={`url(#${id}-wheel)`} stroke-width="6" stroke-linejoin="round">
    <path d={shapes.tyre} fill={value.rubber} stroke={value.rubber} />
    <circle cx="240" cy="124" r="38" fill={value.rim} stroke={value.rim} />
  </g>
  <g fill={value.fenders} stroke={value.fenders} stroke-width="5" stroke-linejoin="round">
    {#each shapes.fenders as path (path)}<path d={path} />{/each}
  </g>
  <g fill={value.hookColour} stroke={value.hookColour} stroke-width="5" stroke-linejoin="round">
    {#each shapes.hooks as hook (hook.transform)}<path d={hook.path} transform={hook.transform} />{/each}
  </g>
  <polyline
    points={shapes.rail}
    fill="none"
    stroke={value.leftTop}
    stroke-width="26"
    stroke-linejoin="round"
    stroke-linecap="round"
  />
  {#each sections as [side, half, colour] (colour)}
    <g clip-path={`url(#${id}-${side})`}
      ><g clip-path={`url(#${id}-${half})`}>
        <polyline
          points={shapes.rail}
          fill="none"
          stroke={value[colour]}
          stroke-width="26"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
      </g></g
    >
  {/each}
  <g fill="var(--board-background, var(--color-elevated))">
    <circle cx="224" cy="124" r="4.2" /><circle cx="256" cy="124" r="4.2" />
  </g>
</svg>
