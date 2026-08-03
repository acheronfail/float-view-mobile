<script lang="ts">
  import type { Snippet } from 'svelte';
  import Menu from './Menu.svelte';

  let {
    headingId,
    title,
    menuLabel,
    menu,
    eyebrow,
    subtitle,
    subtitleClass = 'mt-1 text-sm text-muted',
    titleClass = 'mt-1 text-2xl font-semibold',
    backHref,
    backLabel = 'Back',
    headerHeight = $bindable(0),
    button = $bindable(),
  }: {
    headingId: string;
    title: string;
    menuLabel: string;
    menu: Snippet<[close: () => void]>;
    eyebrow?: string;
    subtitle?: string;
    subtitleClass?: string;
    titleClass?: string;
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
    {#if backHref}
      <a class="text-accent" href={backHref}>{backLabel}</a>
    {/if}
    {#if eyebrow}
      <p class="text-xs font-medium tracking-[0.2em] text-muted uppercase">{eyebrow}</p>
    {/if}
    <h2 id={headingId} class={titleClass}>{title}</h2>
    {#if subtitle}<p class={subtitleClass}>{subtitle}</p>{/if}
  </div>
  <Menu label={menuLabel} bind:button>
    {#snippet children(closeMenu)}
      {@render menu(closeMenu)}
    {/snippet}
  </Menu>
</header>
