<script lang="ts">
  import { bottomTabForRoute, bottomTabs } from './navigation';

  let { routeId }: { routeId: string | null } = $props();
  const activeTab = $derived(bottomTabForRoute(routeId));
</script>

{#if activeTab}
  <nav
    aria-label="Primary"
    data-app-navigation="bottom-tabs"
    class="fixed inset-x-0 bottom-0 z-10 border-t border-outline bg-surface/95 backdrop-blur"
  >
    <div class="bottom-tabs-safe-area mx-auto grid max-w-3xl grid-cols-4">
      {#each bottomTabs as tab (tab.href)}
        <a
          href={tab.href}
          aria-current={activeTab === tab ? 'page' : undefined}
          class="my-2 flex min-h-14 flex-col items-center justify-center gap-1 rounded-xl px-2 text-xs font-medium"
          class:bg-accent-soft={activeTab === tab}
          class:text-on-accent-soft={activeTab === tab}
          class:text-muted={activeTab !== tab}
        >
          <span class="text-2xl">{tab.emoji}</span>
          <span>{tab.label}</span>
        </a>
      {/each}
    </div>
  </nav>
{/if}
