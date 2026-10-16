<script lang="ts">
  import { navigationLink } from './navigation-link';
  import type { Snippet } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';

  type ListItemProps = {
    title: string;
    subtitle: string;
    href?: string;
    trailing?: Snippet;
    onactivate?: Activation;
    disabled?: boolean;
  };

  let { title, subtitle, href, trailing, onactivate, disabled = false }: ListItemProps = $props();
  const interactiveClass =
    'flex w-full min-w-0 touch-manipulation items-center justify-between gap-3 rounded-lg border border-transparent bg-elevated p-3 text-left transition-colors motion-feedback motion-reduce:transition-none hover:border-outline hover:bg-surface active:border-accent active:bg-accent-soft focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-transparent disabled:hover:bg-elevated';
</script>

{#if href}
  <a class={interactiveClass} {href} use:navigationLink={{ scrollable: true }}>
    <span class="block min-w-0 flex-1">
      <span class="block font-medium">{title}</span>
      <span class="text-sm text-muted">{subtitle}</span>
    </span>
    {#if trailing}{@render trailing()}{/if}
  </a>
{:else if onactivate}
  <button
    type="button"
    class={interactiveClass}
    {disabled}
    onpointerdown={(event) => activateFromPointer(event, onactivate)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onactivate)}
  >
    <span class="block min-w-0 flex-1">
      <span class="block font-medium">{title}</span>
      <span class="text-sm text-muted">{subtitle}</span>
    </span>
    {#if trailing}{@render trailing()}{/if}
  </button>
{:else}
  <div class="flex min-w-0 items-center justify-between gap-3 rounded-lg bg-elevated p-3">
    <div class="min-w-0 flex-1">
      <span class="block font-medium">{title}</span>
      <span class="text-sm text-muted">{subtitle}</span>
    </div>
    {#if trailing}{@render trailing()}{/if}
  </div>
{/if}
