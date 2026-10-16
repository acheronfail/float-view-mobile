<script lang="ts">
  import { navigationLink } from '$lib/ui/navigation-link';
  import { connectionHref } from './connection-navigation';
  import type { ActiveConnection } from '$lib/backend/contract';
  import Icon from '$lib/ui/Icon.svelte';
  import { bottomTabForRoute, bottomTabs } from './navigation';

  let { routeId, connection }: { routeId: string | null; connection?: ActiveConnection | null } = $props();
  const activeTab = $derived(bottomTabForRoute(routeId));
</script>

{#if activeTab}
  <nav
    aria-label="Primary"
    data-app-navigation="bottom-tabs"
    class="fixed inset-x-0 bottom-0 z-10 border-t border-outline bg-surface/95 backdrop-blur"
  >
    <div class="bottom-tabs-safe-area mx-auto grid max-w-3xl grid-cols-3">
      {#each bottomTabs as tab (tab.href)}
        <a
          href={tab === bottomTabs[0] ? (connectionHref(connection) ?? tab.href) : tab.href}
          use:navigationLink
          aria-current={activeTab === tab ? 'page' : undefined}
          class="motion-feedback my-2 flex min-h-14 touch-manipulation flex-col items-center justify-center gap-1 rounded-xl px-2 text-xs font-medium transition-colors motion-reduce:transition-none"
          class:bg-accent-soft={activeTab === tab}
          class:text-on-accent-soft={activeTab === tab}
          class:text-muted={activeTab !== tab}
        >
          <Icon name={tab.icon} class="text-2xl" />
          <span>{tab === bottomTabs[0] && connection ? 'board' : tab.label}</span>
        </a>
      {/each}
    </div>
  </nav>
{/if}
