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
  const menuId = $props.id();
  const menuGroups = $derived.by(() => {
    const groups: { label?: string; blocks: { reason?: string; actions: AppHeaderAction[] }[] }[] = [];
    for (const action of actions ?? []) {
      let group = groups.at(-1);
      if (!group || group.label !== action.group) {
        group = { label: action.group, blocks: [] };
        groups.push(group);
      }
      const reason = action.disabled ? action.unavailableReason : undefined;
      const previous = group.blocks.at(-1);
      if (previous && previous.reason === reason) previous.actions.push(action);
      else group.blocks.push({ reason, actions: [action] });
    }
    return groups;
  });

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
  class="app-page-header sticky z-20 flex shrink-0 items-start justify-between gap-4 bg-canvas"
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
      <a
        class="app-back-control inline-flex min-h-11 touch-manipulation items-center text-accent"
        href={backHref}
        use:navigationLink
      >
        <Icon name="arrow-left" class="mr-1 text-xl" />
        <span>{backLabel}</span>
      </a>
    {/if}
    <h1 id={headingId} class={titleClass} {title}>{title}</h1>
    {#if subtitleSnippet}
      <p class={subtitleClass}>{@render subtitleSnippet()}</p>
    {:else if subtitle}
      <p class={subtitleClass}>{subtitle}</p>
    {/if}
  </div>
  <div class="app-header-actions flex shrink-0 items-center gap-2">
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
              {#each menuGroups as group, groupIndex (group.blocks[0].actions[0].label)}
                {#if groupIndex > 0}
                  <div role="separator" class="border-t border-outline"></div>
                {/if}
                <section>
                  {#if group.label}
                    <h2 class="sticky top-0 z-20 bg-elevated px-4 pt-3 pb-1 text-xs font-semibold text-muted">
                      {group.label}
                    </h2>
                  {/if}
                  {#each group.blocks as block, index (block.actions[0].label)}
                    <div
                      data-unavailable-actions={block.reason ? true : undefined}
                      class:unavailable-actions={!!block.reason}
                    >
                      <div class:opacity-30={!!block.reason} class:under-overlay={!!block.reason}>
                        {#each block.actions as action (action.label)}
                          <MenuItem
                            label={action.label}
                            icon={action.icon}
                            appearance={action.appearance}
                            disabled={action.disabled}
                            checked={action.checked}
                            pending={action.pending}
                            descriptionId={block.reason ? `${menuId}-unavailable-${groupIndex}-${index}` : undefined}
                            onactivate={() => action.onactivate(closeMenu)}
                          />
                        {/each}
                      </div>
                      {#if block.reason}
                        <p
                          data-unavailable-overlay
                          id={`${menuId}-unavailable-${groupIndex}-${index}`}
                          class="unavailable-explanation pointer-events-none sticky top-10 bottom-2 z-10 col-start-1 row-start-1 mx-3 my-2 max-w-64 self-center p-3 text-center text-sm font-medium"
                        >
                          {block.reason}
                        </p>
                      {/if}
                    </div>
                  {/each}
                </section>
              {/each}
            </div>
          {/snippet}
        </Menu>
      {/if}
    {/if}
  </div>
</header>

<style>
  .unavailable-actions {
    display: grid;
    position: relative;
    isolation: isolate;
  }

  .under-overlay {
    grid-area: 1 / 1;
    filter: blur(1px);
  }

  .unavailable-actions::before {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(transparent, rgb(0 0 0 / 5%) 20%, rgb(0 0 0 / 5%) 80%, transparent);
  }

  .unavailable-explanation {
    background: radial-gradient(ellipse, var(--app-elevated) 25%, transparent 75%);
    text-shadow: 0 0 6px var(--app-elevated);
  }
</style>
