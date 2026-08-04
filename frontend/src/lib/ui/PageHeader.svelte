<script lang="ts">
  import type { AppHeaderAction } from '$lib/app/header';
  import Menu from './Menu.svelte';
  import MenuItem from './MenuItem.svelte';

  let {
    headingId,
    title,
    subtitle,
    subtitleClass = 'text-sm text-muted',
    titleClass = 'text-3xl font-semibold',
    menuLabel,
    actions,
    backHref,
    backLabel = 'Back',
    headerHeight = $bindable(0),
    button = $bindable(),
  }: {
    headingId: string;
    title: string;
    subtitle?: string;
    subtitleClass?: string;
    titleClass?: string;
    menuLabel?: string;
    actions?: AppHeaderAction[];
    backHref?: string;
    backLabel?: string;
    headerHeight?: number;
    button?: HTMLButtonElement;
  } = $props();

  const safeAreaTop = 'max(1.5rem, env(safe-area-inset-top, 0px))';

  function observeHeader(node: HTMLElement) {
    const updateHeight = () => {
      headerHeight = node.getBoundingClientRect().height;
    };
    updateHeight();
    if (typeof ResizeObserver === 'undefined') return;

    const observer = new ResizeObserver(updateHeight);
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }
</script>

<header
  use:observeHeader
  class="sticky z-20 flex shrink-0 items-start justify-between gap-4 border-b border-b-accent-soft bg-canvas"
  style={`top: ${safeAreaTop}`}
>
  <span
    aria-hidden="true"
    class="pointer-events-none absolute right-0 bottom-full left-0 bg-canvas"
    style={`height: ${safeAreaTop}`}
  ></span>
  <div class="min-w-0">
    {#if backHref}<a class="text-accent" href={backHref}>{backLabel}</a>{/if}
    <h1 id={headingId} class={titleClass}>{title}</h1>
    {#if subtitle}<p class={subtitleClass}>{subtitle}</p>{/if}
  </div>
  {#if actions?.length && menuLabel}
    <Menu label={menuLabel} bind:button>
      {#snippet children(closeMenu)}
        {#each actions as action (action.label)}
          <MenuItem
            label={action.label}
            appearance={action.appearance}
            disabled={action.disabled}
            onactivate={() => action.onactivate(closeMenu)}
          />
        {/each}
      {/snippet}
    </Menu>
  {/if}
</header>
