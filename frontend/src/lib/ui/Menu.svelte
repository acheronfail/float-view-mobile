<script lang="ts">
  import { createMotion } from './motion';
  import type { Snippet } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, isPrimaryPointerActivation } from './activation';
  import Icon from './Icon.svelte';

  const { appear: appearSurface, exit: exitSurface } = createMotion();

  let {
    label,
    children,
    button = $bindable(),
    menuOpen = $bindable(false),
  }: {
    label: string;
    children: Snippet<[close: () => void]>;
    button?: HTMLButtonElement;
    menuOpen?: boolean;
  } = $props();

  let menuRoot = $state<HTMLDivElement>();

  function closeMenu() {
    menuOpen = false;
    button?.focus();
  }

  function toggleMenu() {
    menuOpen = !menuOpen;
  }

  function dismissOnOutsidePointer(event: PointerEvent) {
    if (!menuOpen || !menuRoot || !isPrimaryPointerActivation(event)) return;
    if (!(event.target instanceof Node) || !menuRoot.contains(event.target)) closeMenu();
  }
</script>

<svelte:window onpointerdown={dismissOnOutsidePointer} />

<div bind:this={menuRoot} class="relative shrink-0">
  <button
    bind:this={button}
    type="button"
    class="grid h-11 w-11 touch-manipulation place-items-center rounded-full border border-outline bg-surface"
    aria-label={label}
    aria-expanded={menuOpen}
    onpointerdown={(event) => activateFromPointer(event, toggleMenu)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, toggleMenu)}
  >
    <Icon name="ellipsis-vertical" class="text-xl" />
  </button>
  {#if menuOpen}
    <div
      in:appearSurface={'menu'}
      out:exitSurface
      class="absolute top-12 right-0 z-30 min-w-52 overflow-hidden rounded-xl border border-outline bg-elevated shadow-xl"
    >
      {@render children(closeMenu)}
    </div>
  {/if}
</div>
