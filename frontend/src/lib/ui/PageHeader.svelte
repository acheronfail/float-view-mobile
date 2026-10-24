<script lang="ts">
  import { navigationLink } from './navigation-link';
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
    accessory,
    subtitleClass = 'text-sm text-muted',
    titleClass = 'text-3xl font-semibold',
    bottomBorder = true,
    menuLabel,
    actions,
    menuPresentation,
    menuUnavailable = false,
    menuWarning,
    menuOpen = $bindable(false),
    backHref,
    backLabel = 'Back',
    headerHeight = $bindable(0),
    button = $bindable(),
  }: {
    headingId: string;
    title: string;
    subtitle?: string;
    subtitleSnippet?: Snippet;
    accessory?: Snippet;
    subtitleClass?: string;
    titleClass?: string;
    bottomBorder?: boolean;
    menuLabel?: string;
    actions?: AppHeaderAction[];
    menuPresentation?: 'overlay';
    menuUnavailable?: boolean;
    menuWarning?: string;
    menuOpen?: boolean;
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
  <div class="min-w-0 flex-1">
    {#if backHref}
      <a class="inline-flex min-h-11 touch-manipulation items-center text-accent" href={backHref} use:navigationLink>
        <Icon name="arrow-left" class="mr-1 text-xl" />
        {backLabel}
      </a>
    {/if}
    <h1 id={headingId} class={titleClass} {title}>{title}</h1>
    {#if subtitleSnippet}
      <p class={subtitleClass}>{@render subtitleSnippet()}</p>
    {:else if subtitle}
      <p class={subtitleClass}>{subtitle}</p>
    {/if}
  </div>
  <div class="flex shrink-0 items-center gap-2">
    {#if accessory}{@render accessory()}{/if}
    {#if actions?.length && menuLabel}
      {#if menuPresentation === 'overlay'}
        <HeaderActionOverlay
          label={menuLabel}
          subtitle={title}
          {actions}
          unavailable={menuUnavailable}
          warning={menuWarning}
          bind:button
          bind:open={menuOpen}
        />
      {:else}
        <Menu label={menuLabel} bind:button bind:menuOpen>
          {#snippet children(closeMenu)}
            <div
              data-header-menu
              class="max-h-[calc(100dvh-max(1.5rem,env(safe-area-inset-top,0px))-3rem-max(1rem,env(safe-area-inset-bottom,0px)))] touch-pan-y overflow-y-auto overscroll-contain"
            >
              {#each actions as action, index (action.label)}
                {#if action.group && action.group !== actions[index - 1]?.group}
                  {#if index > 0}
                    <div role="separator" class="border-t border-outline"></div>
                  {/if}
                  <h2 class="sticky top-0 z-10 bg-elevated px-4 pt-3 pb-1 text-xs font-semibold text-muted">
                    {action.group}
                  </h2>
                {/if}
                {#if action.groupDescription}
                  <p class="max-w-72 px-4 pt-2 pb-1 text-sm text-muted">{action.groupDescription}</p>
                {/if}
                <MenuItem
                  label={action.label}
                  icon={action.icon}
                  appearance={action.appearance}
                  disabled={action.disabled}
                  checked={action.checked}
                  pending={action.pending}
                  onactivate={() => action.onactivate(closeMenu)}
                />
              {/each}
            </div>
          {/snippet}
        </Menu>
      {/if}
    {/if}
  </div>
</header>
