<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AppHeaderAction } from '$lib/app/header';
  import Icon from './Icon.svelte';
  import Menu from './Menu.svelte';
  import HeaderActionOverlay from '$lib/app/HeaderActionOverlay.svelte';
  import MenuItem from './MenuItem.svelte';

  let {
    headingId,
    title,
    subtitle,
    subtitleSnippet,
    subtitleClass = 'text-sm text-muted',
    titleClass = 'text-3xl font-semibold',
    bottomBorder = true,
    menuLabel,
    actions,
    menuPresentation,
    menuUnavailable = false,
    backHref,
    backLabel = 'Back',
    headerHeight = $bindable(0),
    button = $bindable(),
  }: {
    headingId: string;
    title: string;
    subtitle?: string;
    subtitleSnippet?: Snippet;
    subtitleClass?: string;
    titleClass?: string;
    bottomBorder?: boolean;
    menuLabel?: string;
    actions?: AppHeaderAction[];
    menuPresentation?: 'overlay';
    menuUnavailable?: boolean;
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
  class="sticky z-20 flex shrink-0 items-start justify-between gap-4 bg-canvas"
  class:border-b={bottomBorder}
  class:border-b-accent-soft={bottomBorder}
  style={`top: ${safeAreaTop}`}
>
  <span
    aria-hidden="true"
    class="pointer-events-none absolute right-0 bottom-full left-0 bg-canvas"
    style={`height: ${safeAreaTop}`}
  ></span>
  <div class="min-w-0">
    {#if backHref}
      <a class="inline-flex items-center text-accent" href={backHref}>
        <Icon name="arrow-left" class="mr-1 text-xl" />
        {backLabel}
      </a>
    {/if}
    <h1 id={headingId} class={titleClass}>{title}</h1>
    {#if subtitleSnippet}
      <p class={subtitleClass}>{@render subtitleSnippet()}</p>
    {:else if subtitle}
      <p class={subtitleClass}>{subtitle}</p>
    {/if}
  </div>
  {#if actions?.length && menuLabel}
    {#if menuPresentation === 'overlay'}
      <HeaderActionOverlay label={menuLabel} subtitle={title} {actions} unavailable={menuUnavailable} bind:button />
    {:else}
      <Menu label={menuLabel} bind:button>
        {#snippet children(closeMenu)}
          {#each actions as action (action.label)}
            <MenuItem
              label={action.label}
              appearance={action.appearance}
              disabled={action.disabled}
              checked={action.checked}
              pending={action.pending}
              onactivate={() => action.onactivate(closeMenu)}
            />
          {/each}
        {/snippet}
      </Menu>
    {/if}
  {/if}
</header>
