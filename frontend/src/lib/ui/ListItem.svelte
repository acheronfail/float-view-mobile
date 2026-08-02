<script lang="ts">
  import type { Snippet } from 'svelte';

  type ListItemProps = {
    title: string;
    subtitle: string;
    href?: string;
    trailing?: Snippet;
  };

  let { title, subtitle, href, trailing }: ListItemProps = $props();
</script>

{#if href}
  <a class="flex min-w-0 items-center justify-between gap-3 rounded-lg bg-elevated p-3 text-left" {href}>
    <div class="min-w-0 flex-1">
      <span class="block font-medium">{title}</span>
      <span class="text-sm text-muted">{subtitle}</span>
    </div>
    {#if trailing}{@render trailing()}{/if}
  </a>
{:else}
  <div class="flex min-w-0 items-center justify-between gap-3 rounded-lg bg-elevated p-3">
    <div class="min-w-0 flex-1">
      <span class="block font-medium">{title}</span>
      <span class="text-sm text-muted">{subtitle}</span>
    </div>
    {#if trailing}{@render trailing()}{/if}
  </div>
{/if}
