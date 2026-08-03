<script lang="ts">
  import type { Snippet } from 'svelte';
  import Menu from './Menu.svelte';

  let {
    headingId,
    title,
    subtitle,
    sticky = false,
    menuLabel,
    menu,
    headerHeight = $bindable(0),
    button = $bindable(),
  }: {
    headingId: string;
    title: string;
    subtitle?: string;
    sticky?: boolean;
    menuLabel?: string;
    menu?: Snippet<[close: () => void]>;
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
  class={sticky
    ? 'sticky z-20 flex shrink-0 items-start justify-between gap-4 border-b border-b-accent-soft bg-canvas'
    : 'mb-5'}
  style={sticky ? `top: ${safeAreaTop}` : undefined}
>
  {#if sticky}
    <span
      aria-hidden="true"
      class="pointer-events-none absolute right-0 bottom-full left-0 bg-canvas"
      style={`height: ${safeAreaTop}`}
    ></span>
  {/if}
  <div class="min-w-0">
    <h1 id={headingId} class="text-3xl font-semibold">{title}</h1>
    {#if subtitle}<p class="text-sm text-muted">{subtitle}</p>{/if}
  </div>
  {#if menu && menuLabel}
    <Menu label={menuLabel} bind:button>
      {#snippet children(closeMenu)}
        {@render menu(closeMenu)}
      {/snippet}
    </Menu>
  {/if}
</header>
